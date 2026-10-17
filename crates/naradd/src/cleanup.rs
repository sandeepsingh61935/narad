//! Local AI cleanup pipeline heuristics and grounding guard (FR-TXT-6/7/8).
//!
//! Mode-specific transformations (Clean, Bullet, Email) with Plain and Code modes
//! disabled. Evaluated by a deterministic GroundingGuard (length ratio and novel-word
//! thresholds) that falls back to pre-cleanup text with `flagged = true` upon violation.

use std::collections::HashSet;

/// Result of applying cleanup and passing through the grounding guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupResult {
    pub text: String,
    pub flagged: bool,
    pub applied: bool,
}

/// Grounding guard protecting user intent against ungrounded hallucinations or bloating.
#[derive(Debug, Clone, PartialEq)]
pub struct GroundingGuard {
    pub max_length_ratio: f64,
    pub min_cushion_chars: usize,
    pub max_novel_word_ratio: f64,
}

impl Default for GroundingGuard {
    fn default() -> Self {
        Self {
            max_length_ratio: 2.5,
            min_cushion_chars: 40,
            max_novel_word_ratio: 0.35,
        }
    }
}

/// Grounding check violation details.
#[derive(Debug, Clone, PartialEq)]
pub enum GroundingViolation {
    LengthExceeded {
        raw_len: usize,
        cleaned_len: usize,
        max_allowed: usize,
    },
    NovelWordsExceeded {
        novel_ratio: f64,
        max_ratio: f64,
        novel_words: Vec<String>,
    },
}

impl std::fmt::Display for GroundingViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LengthExceeded {
                raw_len,
                cleaned_len,
                max_allowed,
            } => write!(
                f,
                "Cleaned text length exceeds threshold: {cleaned_len} chars vs {raw_len} raw (max allowed: {max_allowed})"
            ),
            Self::NovelWordsExceeded {
                novel_ratio,
                max_ratio,
                novel_words,
            } => write!(
                f,
                "Novel word ratio exceeds threshold: {novel_ratio:.2} (max {max_ratio:.2}), novel words: {novel_words:?}"
            ),
        }
    }
}

impl std::error::Error for GroundingViolation {}

/// Whitelist of structural boilerplate words per mode.
pub fn boilerplate_for_mode(mode: &str) -> &'static [&'static str] {
    match mode.to_ascii_lowercase().as_str() {
        "email" => &[
            "hi",
            "hello",
            "dear",
            "best",
            "regards",
            "thanks",
            "thank",
            "you",
            "sender",
            "sincerely",
            "cheers",
        ],
        "bullet" | "notes" => &["•", "-", "*"],
        _ => &[],
    }
}

/// Extract normalized word tokens for grounding analysis.
pub fn tokenize_words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|tok| {
            let trimmed = tok.trim_matches(|c: char| {
                !c.is_alphanumeric() && c != '•' && c != '-' && c != '*'
            });
            trimmed.to_lowercase()
        })
        .filter(|tok| !tok.is_empty())
        .collect()
}

impl GroundingGuard {
    /// Validates `cleaned` against `raw` under `mode`.
    pub fn check_grounding(
        &self,
        raw: &str,
        cleaned: &str,
        mode: &str,
    ) -> Result<(), GroundingViolation> {
        // 1. Length ratio check
        let raw_len = raw.chars().count();
        let cleaned_len = cleaned.chars().count();
        let max_allowed = ((raw_len as f64 * self.max_length_ratio).ceil() as usize)
            .max(raw_len.saturating_add(self.min_cushion_chars));

        if cleaned_len > max_allowed {
            return Err(GroundingViolation::LengthExceeded {
                raw_len,
                cleaned_len,
                max_allowed,
            });
        }

        // 2. Novel word ratio check
        let raw_words: HashSet<String> = tokenize_words(raw).into_iter().collect();
        let cleaned_words = tokenize_words(cleaned);
        let boilerplate = boilerplate_for_mode(mode);

        let mut novel_words = Vec::new();
        let mut non_boilerplate_count = 0;

        for word in cleaned_words {
            if boilerplate.iter().any(|&bp| bp.eq_ignore_ascii_case(&word)) {
                continue;
            }
            non_boilerplate_count += 1;
            if !raw_words.contains(&word) {
                novel_words.push(word);
            }
        }

        if non_boilerplate_count > 0 {
            let novel_ratio = novel_words.len() as f64 / non_boilerplate_count as f64;
            if novel_ratio > self.max_novel_word_ratio {
                return Err(GroundingViolation::NovelWordsExceeded {
                    novel_ratio,
                    max_ratio: self.max_novel_word_ratio,
                    novel_words,
                });
            }
        }

        Ok(())
    }

    /// Evaluates `cleaned` and returns `CleanupResult`. Falls back to `raw` on violation.
    pub fn validate(&self, raw: &str, cleaned: &str, mode: &str) -> CleanupResult {
        match self.check_grounding(raw, cleaned, mode) {
            Ok(()) => CleanupResult {
                text: cleaned.to_string(),
                flagged: false,
                applied: true,
            },
            Err(_) => CleanupResult {
                text: raw.to_string(),
                flagged: true,
                applied: false,
            },
        }
    }
}

/// Applies mode cleanup heuristics and validates with default GroundingGuard.
pub fn apply_cleanup(raw: &str, mode: &str) -> CleanupResult {
    let guard = GroundingGuard::default();
    apply_cleanup_with_guard(raw, mode, &guard)
}

/// Applies mode cleanup heuristics and validates with the provided GroundingGuard.
pub fn apply_cleanup_with_guard(raw: &str, mode: &str, guard: &GroundingGuard) -> CleanupResult {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return CleanupResult {
            text: raw.to_string(),
            flagged: false,
            applied: false,
        };
    }

    if mode.eq_ignore_ascii_case("plain") || mode.eq_ignore_ascii_case("code") {
        return CleanupResult {
            text: raw.to_string(),
            flagged: false,
            applied: false,
        };
    }

    let cleaned = match mode.to_ascii_lowercase().as_str() {
        "clean" => clean_text(trimmed),
        "bullet" | "notes" => bullet_text(trimmed),
        "email" => email_text(trimmed),
        _ => clean_text(trimmed),
    };

    guard.validate(raw, &cleaned, mode)
}

/// Applies default GroundingGuard validation to candidate text.
pub fn apply_guard(raw: &str, cleaned: &str, mode: &str) -> CleanupResult {
    GroundingGuard::default().validate(raw, cleaned, mode)
}

/// Collapse adjacent orphaned commas, spaces before commas, and orphaned punctuation.
fn collapse_orphaned_punctuation(text: &str) -> String {
    let mut s = text.to_string();
    // Collapse spaces before commas: " ," -> ","
    while s.contains(" ,") {
        s = s.replace(" ,", ",");
    }
    // Collapse adjacent commas (with or without spaces in between)
    while s.contains(",,") || s.contains(", ,") || s.contains(",  ,") {
        s = s.replace(", ,", ",");
        s = s.replace(",  ,", ",");
        s = s.replace(",,", ",");
    }
    // Collapse comma followed directly or with space by sentence punctuation
    s = s.replace(",.", ".");
    s = s.replace(", .", ".");
    s = s.replace(",!", "!");
    s = s.replace(", !", "!");
    s = s.replace(",?", "?");
    s = s.replace(", ?", "?");

    // Trim leading/trailing orphan commas and whitespace
    let trimmed = s.trim();
    let trimmed = trimmed
        .trim_start_matches(|c: char| c == ',' || c.is_whitespace())
        .trim_end_matches(|c: char| c == ',' || c.is_whitespace());
    trimmed.to_string()
}

const SIGNOFF_KEYWORDS: &[&str] = &["best", "regards", "sincerely", "cheers", "thanks"];

/// Strip filler phrases and filler words from a single line using safe token window matching.
fn strip_fillers_line(raw: &str) -> String {
    let tokens: Vec<&str> = raw.split_whitespace().collect();
    if tokens.is_empty() {
        return String::new();
    }

    let word_fillers = ["um", "uh", "er", "ah", "basically"];
    let signoff_keywords = SIGNOFF_KEYWORDS;
    let mut filtered_tokens: Vec<&str> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    let mut any_removed = false;
    while i < tokens.len() {
        let core0 = tokens[i]
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();

        // Check 2-token window for multi-word fillers: "you know", "i mean"
        if i + 1 < tokens.len() {
            let core1 = tokens[i + 1]
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();
            if (core0 == "you" && core1 == "know") || (core0 == "i" && core1 == "mean") {
                any_removed = true;
                i += 2;
                continue;
            }
        }

        // Check 1-token filler
        if word_fillers.contains(&core0.as_str()) {
            any_removed = true;
            i += 1;
            continue;
        }

        filtered_tokens.push(tokens[i]);
        i += 1;
    }

    if !any_removed {
        return raw.trim().to_string();
    }

    let joined = filtered_tokens.join(" ");
    let mut collapsed = collapse_orphaned_punctuation(&joined);

    // If the line ended with a signoff followed by comma, preserve the comma
    if let Some(last_tok) = filtered_tokens.last() {
        let core = last_tok.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
        if signoff_keywords.contains(&core.as_str()) && raw.trim_end().ends_with(',') && !collapsed.ends_with(',') {
            collapsed.push(',');
        }
    }

    collapsed
}

/// Strip filler phrases and filler words from raw text using safe token window matching.
fn strip_fillers(raw: &str) -> String {
    if raw.contains('\n') {
        let lines: Vec<String> = raw.lines().map(strip_fillers_line).collect();
        lines.join("\n")
    } else {
        strip_fillers_line(raw)
    }
}

/// Remove consecutive duplicate words from a single line.
fn remove_repetitions_line(text: &str) -> String {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let mut result: Vec<&str> = Vec::new();
    for tok in tokens {
        let core = tok.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
        if let Some(prev) = result.last() {
            let prev_core = prev.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
            if !core.is_empty() && core == prev_core {
                continue;
            }
        }
        result.push(tok);
    }
    result.join(" ")
}

/// Remove consecutive duplicate words.
fn remove_repetitions(text: &str) -> String {
    if text.contains('\n') {
        let lines: Vec<String> = text.lines().map(remove_repetitions_line).collect();
        lines.join("\n")
    } else {
        remove_repetitions_line(text)
    }
}

/// Standardizes capitalization and ensures sentence-ending punctuation.
///
/// Guard: require `.` to be followed by whitespace or end of string before capitalizing
/// the next character, preventing `narad.org` -> `narad.Org` or `main.rs` -> `main.Rs`.
fn standardize_sentence(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }

    let mut chars_iter = text.chars();
    let first = chars_iter.next().unwrap();
    let mut result = String::new();
    result.extend(first.to_uppercase());
    result.push_str(chars_iter.as_str());

    let char_vec: Vec<char> = result.chars().collect();
    let mut final_text = String::new();
    let mut cap_next = false;
    for (i, &c) in char_vec.iter().enumerate() {
        if cap_next && c.is_alphabetic() {
            final_text.extend(c.to_uppercase());
            cap_next = false;
        } else {
            if c == '.' {
                let mut next_i = i + 1;
                while next_i < char_vec.len() && matches!(char_vec[next_i], '"' | '\'' | ')' | ']') {
                    next_i += 1;
                }
                if next_i == char_vec.len() || char_vec[next_i].is_whitespace() {
                    cap_next = true;
                }
            } else if c == '!' || c == '?' {
                cap_next = true;
            } else if !c.is_whitespace() && !matches!(c, '"' | '\'' | ')' | ']') {
                cap_next = false;
            }
            final_text.push(c);
        }
    }

    let trimmed_punct = final_text.trim_end_matches(|c: char| matches!(c, '"' | '\'' | ')' | ']'));
    if !trimmed_punct.ends_with('.') && !trimmed_punct.ends_with('!') && !trimmed_punct.ends_with('?') {
        final_text.push('.');
    }

    final_text
}

/// Clean mode heuristics: strips fillers, repetitions, and standardizes spacing/punctuation.
pub fn clean_text(raw: &str) -> String {
    let without_fillers = strip_fillers(raw);
    let without_repetitions = remove_repetitions(&without_fillers);
    standardize_sentence(&without_repetitions)
}

/// Splits input into sentences or newline items.
fn split_into_items(raw: &str) -> Vec<String> {
    if raw.contains('\n') {
        raw.lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    } else {
        let mut items = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = raw.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            current.push(c);
            if (c == '.' || c == '!' || c == '?')
                && (i + 1 == chars.len() || chars[i + 1].is_whitespace())
            {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    items.push(trimmed);
                }
                current.clear();
            }
            i += 1;
        }
        let trimmed = current.trim().to_string();
        if !trimmed.is_empty() {
            items.push(trimmed);
        }
        items
    }
}

/// Bullet mode heuristics: structures items into bullets (`• ...`).
pub fn bullet_text(raw: &str) -> String {
    let items = split_into_items(raw);
    let bullets: Vec<String> = items
        .into_iter()
        .map(|item| {
            let stripped = item
                .trim_start_matches(|c: char| c == '•' || c == '-' || c == '*' || c.is_whitespace())
                .trim();
            if stripped.is_empty() {
                return String::new();
            }
            let mut chars = stripped.chars();
            let first = chars.next().unwrap();
            let mut capitalized = String::new();
            capitalized.extend(first.to_uppercase());
            capitalized.push_str(chars.as_str());
            format!("• {capitalized}")
        })
        .filter(|b| !b.is_empty())
        .collect();

    bullets.join("\n")
}

/// Email mode heuristics: structures into greeting, body, and sign-off.
pub fn email_text(raw: &str) -> String {
    let mut body = clean_text(raw);
    if body.is_empty() {
        return String::new();
    }

    let greeting_keywords = ["hi", "hello", "dear"];
    let first_token = body
        .split_whitespace()
        .next()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .unwrap_or_default();
    let has_greeting = greeting_keywords.contains(&first_token.as_str());

    let signoff_keywords = SIGNOFF_KEYWORDS;
    let lines: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
    let mut is_personal_name_final = false;
    let has_signoff = if let Some(&last_line) = lines.last() {
        let last_tokens: Vec<String> = last_line
            .split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
            .filter(|w| !w.is_empty())
            .collect();
        if last_tokens
            .iter()
            .rev()
            .take(3)
            .any(|w| signoff_keywords.contains(&w.as_str()))
        {
            true
        } else if (1..=2).contains(&last_tokens.len()) {
            if let Some(&prev_line) = lines.iter().rev().nth(1) {
                let prev_tokens: Vec<String> = prev_line
                    .split_whitespace()
                    .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
                    .filter(|w| !w.is_empty())
                    .collect();
                if prev_tokens
                    .iter()
                    .rev()
                    .take(3)
                    .any(|w| signoff_keywords.contains(&w.as_str()))
                {
                    is_personal_name_final = true;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    if has_signoff && is_personal_name_final && !raw.trim_end().ends_with('.') && body.ends_with('.') {
        body.pop();
    }

    match (has_greeting, has_signoff) {
        (true, true) => body,
        (true, false) => format!("{body}\n\nBest,\nSender"),
        (false, true) => format!("Hi,\n\n{body}"),
        (false, false) => format!("Hi,\n\n{body}\n\nBest,\nSender"),
    }
}

/// Extracts spoken mode prefix from raw speech text (FR-ACT-5).
///
/// Recognizes prefixes case-insensitively:
/// - `"plain:"` / `"plain "`
/// - `"clean:"` / `"clean "`
/// - `"email:"` / `"email "`
/// - `"bullet:"` / `"bullet "` / `"bullet list:"` / `"bullet points:"`
/// - `"code:"` / `"code "`
///
/// Returns `Some((mode_name, remainder))` where `remainder` has leading/trailing
/// punctuation and whitespace trimmed, and `mode_name` is `"plain"`, `"clean"`,
/// `"email"`, `"bullet"`, or `"code"`.
/// Returns `None` if no recognized prefix matches.
pub fn extract_spoken_mode_prefix(text: &str) -> Option<(&'static str, &str)> {
    let trimmed_start = text.trim_start();
    let lower = trimmed_start.to_ascii_lowercase();

    const PREFIX_TABLE: &[(&str, &str)] = &[
        ("bullet points:", "bullet"),
        ("bullet points ", "bullet"),
        ("bullet list:", "bullet"),
        ("bullet list ", "bullet"),
        ("bullet:", "bullet"),
        ("bullet ", "bullet"),
        ("plain:", "plain"),
        ("plain ", "plain"),
        ("clean:", "clean"),
        ("clean ", "clean"),
        ("email:", "email"),
        ("email ", "email"),
        ("code:", "code"),
        ("code ", "code"),
    ];

    for &(prefix, mode) in PREFIX_TABLE {
        if lower.starts_with(prefix) {
            let rem = &trimmed_start[prefix.len()..];
            let trimmed_start_rem = rem.trim_start_matches(|c: char| {
                c.is_whitespace()
                    || matches!(c, ':' | ',' | ';' | '-' | '.' | '!' | '?' | '•' | '—' | '–')
            });
            let trimmed_rem = if mode == "code" {
                trimmed_start_rem.trim_end_matches(|c: char| {
                    c.is_whitespace() || matches!(c, ':' | ',' | '-' | '.' | '!' | '?')
                })
            } else {
                trimmed_start_rem.trim_end_matches(|c: char| {
                    c.is_whitespace() || matches!(c, ':' | ',' | ';' | '-' | '.' | '!' | '?')
                })
            };
            return Some((mode, trimmed_rem));
        }
    }

    None
}

const CONTRACTIONS: &[(&str, &str)] = &[
    ("aren't", "are not"),
    ("can't", "cannot"),
    ("couldn't", "could not"),
    ("didn't", "did not"),
    ("doesn't", "does not"),
    ("don't", "do not"),
    ("hadn't", "had not"),
    ("hasn't", "has not"),
    ("haven't", "have not"),
    ("he'd", "he would"),
    ("he'll", "he will"),
    ("he's", "he is"),
    ("i'd", "I would"),
    ("i'll", "I will"),
    ("i'm", "I am"),
    ("i've", "I have"),
    ("isn't", "is not"),
    ("it'd", "it would"),
    ("it'll", "it will"),
    ("it's", "it is"),
    ("let's", "let us"),
    ("she'd", "she would"),
    ("she'll", "she will"),
    ("she's", "she is"),
    ("shouldn't", "should not"),
    ("that's", "that is"),
    ("there's", "there is"),
    ("they'd", "they would"),
    ("they'll", "they will"),
    ("they're", "they are"),
    ("they've", "they have"),
    ("wasn't", "was not"),
    ("we'd", "we would"),
    ("we'll", "we will"),
    ("we're", "we are"),
    ("we've", "we have"),
    ("weren't", "were not"),
    ("what's", "what is"),
    ("who's", "who is"),
    ("won't", "will not"),
    ("wouldn't", "would not"),
    ("you'd", "you would"),
    ("you'll", "you will"),
    ("you're", "you are"),
    ("you've", "you have"),
];

fn expand_contractions_line(text: &str) -> String {
    let normalized = text.replace('’', "'");
    let mut tokens: Vec<String> = Vec::new();
    for raw_tok in normalized.split_whitespace() {
        let lead_len = raw_tok
            .chars()
            .take_while(|&c| c.is_ascii_punctuation() && c != '\'')
            .map(|c| c.len_utf8())
            .sum::<usize>();
        let (lead, rest) = raw_tok.split_at(lead_len);

        let trail_len = rest
            .chars()
            .rev()
            .take_while(|&c| c.is_ascii_punctuation() && c != '\'')
            .map(|c| c.len_utf8())
            .sum::<usize>();
        let (core, trail) = rest.split_at(rest.len() - trail_len);

        let lower_core = core.to_ascii_lowercase();
        if let Some(&(_, replacement)) = CONTRACTIONS.iter().find(|&&(k, _)| k == lower_core) {
            let is_capitalized = core.chars().next().map_or(false, |c| c.is_uppercase());
            let mut rep = replacement.to_string();
            if is_capitalized {
                if let Some(first) = rep.chars().next() {
                    let mut cap = String::new();
                    cap.extend(first.to_uppercase());
                    cap.push_str(&rep[first.len_utf8()..]);
                    rep = cap;
                }
            }
            tokens.push(format!("{lead}{rep}{trail}"));
        } else {
            tokens.push(raw_tok.to_string());
        }
    }
    tokens.join(" ")
}

fn expand_contractions(text: &str) -> String {
    if text.contains('\n') {
        text.lines()
            .map(expand_contractions_line)
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        expand_contractions_line(text)
    }
}

fn format_formal(text: &str) -> String {
    let expanded = expand_contractions(text);
    let without_fillers = strip_fillers(&expanded);
    standardize_sentence(&without_fillers)
}

const VERBOSE_PHRASES: &[(&str, &str)] = &[
    ("at the present time", "now"),
    ("at this point in time", "now"),
    ("due to the fact that", "because"),
    ("for the purpose of", "for"),
    ("in order to", "to"),
    ("in the event that", "if"),
    ("in light of the fact that", "because"),
    ("it is important to note that", ""),
    ("it should be noted that", ""),
    ("needless to say", ""),
    ("as a matter of fact", ""),
    ("at the end of the day", ""),
];

const FLUFF_WORDS: &[&str] = &[
    "basically",
    "literally",
    "actually",
    "definitely",
    "really",
    "very",
    "quite",
    "simply",
];

fn format_concise_line(text: &str) -> String {
    if text.trim().is_empty() {
        return String::new();
    }
    let mut s = text.to_string();
    for &(phrase, replacement) in VERBOSE_PHRASES {
        let lower = s.to_ascii_lowercase();
        let mut idx = 0;
        let mut result = String::new();
        let mut found = false;
        while let Some(pos) = lower[idx..].find(phrase) {
            found = true;
            let abs_pos = idx + pos;
            result.push_str(&s[idx..abs_pos]);
            let orig_slice = &s[abs_pos..abs_pos + phrase.len()];
            if orig_slice
                .chars()
                .next()
                .map_or(false, |c| c.is_uppercase())
                && !replacement.is_empty()
            {
                let mut cap = String::new();
                let mut chars = replacement.chars();
                if let Some(first) = chars.next() {
                    cap.extend(first.to_uppercase());
                    cap.push_str(chars.as_str());
                }
                result.push_str(&cap);
            } else {
                result.push_str(replacement);
            }
            idx = abs_pos + phrase.len();
        }
        if found {
            result.push_str(&s[idx..]);
            s = result;
        }
    }

    let without_fillers = strip_fillers_line(&s);
    let tokens: Vec<&str> = without_fillers.split_whitespace().collect();
    let filtered: Vec<&str> = tokens
        .into_iter()
        .filter(|tok| {
            let core = tok
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_ascii_lowercase();
            !FLUFF_WORDS.contains(&core.as_str())
        })
        .collect();
    let joined = filtered.join(" ");
    let collapsed = collapse_orphaned_punctuation(&joined);
    let without_rep = remove_repetitions_line(&collapsed);
    standardize_sentence(&without_rep)
}

fn format_concise(text: &str) -> String {
    if text.contains('\n') {
        text.lines()
            .map(format_concise_line)
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        format_concise_line(text)
    }
}

fn to_title_case(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut capitalize_next = true;
    for c in text.chars() {
        if c.is_alphabetic() {
            if capitalize_next {
                result.extend(c.to_uppercase());
                capitalize_next = false;
            } else {
                result.extend(c.to_lowercase());
            }
        } else {
            result.push(c);
            if c.is_whitespace() || c == '-' || c == '/' || c == '_' {
                capitalize_next = true;
            }
        }
    }
    result
}

fn format_as_code(text: &str, instruction: &str) -> String {
    if text.starts_with("```") && text.ends_with("```") {
        return text.to_string();
    }
    if text.contains('\n') || instruction.contains("block") {
        format!("```\n{text}\n```")
    } else if text.starts_with('`') && text.ends_with('`') {
        text.to_string()
    } else {
        format!("`{text}`")
    }
}

fn instruction_has_word(instruction: &str, word: &str) -> bool {
    instruction
        .split(|c: char| !c.is_alphanumeric())
        .any(|w| w.eq_ignore_ascii_case(word))
}

fn instruction_has_phrase(instruction: &str, phrase: &str) -> bool {
    let inst_lower = instruction.to_ascii_lowercase();
    let phrase_lower = phrase.to_ascii_lowercase();
    let mut start = 0;
    while let Some(pos) = inst_lower[start..].find(&phrase_lower) {
        let abs_pos = start + pos;
        let before_ok = abs_pos == 0
            || !inst_lower[..abs_pos]
                .chars()
                .next_back()
                .map_or(false, |c| c.is_alphanumeric());
        let end_pos = abs_pos + phrase_lower.len();
        let after_ok = end_pos == inst_lower.len()
            || !inst_lower[end_pos..]
                .chars()
                .next()
                .map_or(false, |c| c.is_alphanumeric());
        if before_ok && after_ok {
            return true;
        }
        start = abs_pos + phrase_lower.len();
    }
    false
}

/// Dispatches transformation on `selection` based on the intent in `instruction` (FR-INS-6).
pub fn apply_selection_command(selection: &str, instruction: &str) -> String {
    let trimmed_sel = selection.trim();
    if trimmed_sel.is_empty() {
        return selection.to_string();
    }

    if instruction_has_word(instruction, "formal") || instruction_has_word(instruction, "professional") {
        format_formal(trimmed_sel)
    } else if instruction_has_word(instruction, "shorten")
        || instruction_has_word(instruction, "concise")
        || instruction_has_word(instruction, "summarize")
    {
        format_concise(trimmed_sel)
    } else if instruction_has_word(instruction, "bullet")
        || instruction_has_word(instruction, "bullets")
        || instruction_has_word(instruction, "list")
        || instruction_has_word(instruction, "points")
    {
        bullet_text(trimmed_sel)
    } else if instruction_has_word(instruction, "code") || instruction_has_word(instruction, "syntax") {
        format_as_code(trimmed_sel, instruction)
    } else if instruction_has_word(instruction, "email") || instruction_has_word(instruction, "letter") {
        email_text(trimmed_sel)
    } else if instruction_has_word(instruction, "uppercase") || instruction_has_phrase(instruction, "all caps") {
        trimmed_sel.to_uppercase()
    } else if instruction_has_word(instruction, "lowercase") || instruction_has_phrase(instruction, "lower case") {
        trimmed_sel.to_lowercase()
    } else if instruction_has_word(instruction, "capitalize")
        || instruction_has_word(instruction, "capitalized")
        || instruction_has_phrase(instruction, "title case")
        || instruction_has_word(instruction, "titlecase")
    {
        to_title_case(trimmed_sel)
    } else if instruction_has_word(instruction, "clean")
        || instruction_has_word(instruction, "grammar")
        || instruction_has_word(instruction, "fix")
    {
        clean_text(trimmed_sel)
    } else {
        clean_text(trimmed_sel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_mode_leaves_text_untouched() {
        let raw = "um I think basically we should test this";
        let res = apply_cleanup(raw, "plain");
        assert_eq!(res.text, raw);
        assert!(!res.applied);
        assert!(!res.flagged);

        let res_upper = apply_cleanup(raw, "Plain");
        assert_eq!(res_upper.text, raw);
        assert!(!res_upper.applied);
        assert!(!res_upper.flagged);
    }

    #[test]
    fn code_mode_leaves_text_untouched() {
        let raw = "camel case user name period fn test() {}";
        let res = apply_cleanup(raw, "code");
        assert_eq!(res.text, raw);
        assert!(!res.applied);
        assert!(!res.flagged);

        let res_upper = apply_cleanup(raw, "Code");
        assert_eq!(res_upper.text, raw);
        assert!(!res_upper.applied);
        assert!(!res_upper.flagged);
    }

    #[test]
    fn clean_mode_removes_fillers_and_passes_guard() {
        let raw = "um I think we should uh go to the office you know basically";
        let res = apply_cleanup(raw, "clean");
        assert_eq!(res.text, "I think we should go to the office.");
        assert!(res.applied);
        assert!(!res.flagged);

        let raw_rep = "we we need to um talk";
        let res_rep = apply_cleanup(raw_rep, "clean");
        assert_eq!(res_rep.text, "We need to talk.");
        assert!(res_rep.applied);
        assert!(!res_rep.flagged);
    }

    #[test]
    fn bullet_mode_structures_sentences_into_bullets_and_passes_guard() {
        let raw = "Review the PR. Test the daemon. Deploy to staging.";
        let res = apply_cleanup(raw, "bullet");
        assert_eq!(
            res.text,
            "• Review the PR.\n• Test the daemon.\n• Deploy to staging."
        );
        assert!(res.applied);
        assert!(!res.flagged);
    }

    #[test]
    fn email_mode_structures_text_into_email_and_passes_guard() {
        let raw = "The meeting is rescheduled to tomorrow at two.";
        let res = apply_cleanup(raw, "email");
        assert_eq!(
            res.text,
            "Hi,\n\nThe meeting is rescheduled to tomorrow at two.\n\nBest,\nSender"
        );
        assert!(res.applied);
        assert!(!res.flagged);
    }

    #[test]
    fn novel_word_hallucination_triggers_grounding_guard() {
        let guard = GroundingGuard::default();
        let raw = "The quick brown fox jumps over the lazy dog in the sunny park today.";
        let hallucinated =
            "The quick brown cat leaps above the sleepy wolf in the dark forest tonight.";

        let check = guard.check_grounding(raw, hallucinated, "clean");
        assert!(matches!(
            check,
            Err(GroundingViolation::NovelWordsExceeded { .. })
        ));

        let res = guard.validate(raw, hallucinated, "clean");
        assert_eq!(res.text, raw);
        assert!(res.flagged);
        assert!(!res.applied);

        let res_helper = apply_guard(raw, hallucinated, "clean");
        assert_eq!(res_helper.text, raw);
        assert!(res_helper.flagged);
        assert!(!res_helper.applied);
    }

    #[test]
    fn length_ratio_violation_triggers_grounding_guard() {
        let guard = GroundingGuard::default();
        let raw = "Short update.";
        let bloated = "Short update. ".repeat(30);

        let check = guard.check_grounding(raw, &bloated, "clean");
        assert!(matches!(
            check,
            Err(GroundingViolation::LengthExceeded { .. })
        ));

        let res = guard.validate(raw, &bloated, "clean");
        assert_eq!(res.text, raw);
        assert!(res.flagged);
        assert!(!res.applied);

        let res_helper = apply_guard(raw, &bloated, "clean");
        assert_eq!(res_helper.text, raw);
        assert!(res_helper.flagged);
        assert!(!res_helper.applied);
    }

    #[test]
    fn empty_input_returns_untouched() {
        let res = apply_cleanup("", "clean");
        assert_eq!(res.text, "");
        assert!(!res.applied);
        assert!(!res.flagged);

        let res_ws = apply_cleanup("   \n\t", "email");
        assert_eq!(res_ws.text, "   \n\t");
        assert!(!res_ws.applied);
        assert!(!res_ws.flagged);
    }

    #[test]
    fn email_boilerplate_words_are_whitelisted() {
        let guard = GroundingGuard::default();
        let raw =
            "The team met yesterday to discuss roadmap milestones and delivery schedule.";
        let cleaned =
            "Hi,\n\nDear team, the team met yesterday to discuss roadmap milestones and delivery schedule.\n\nThanks, best regards,\nSender";
        assert!(guard.check_grounding(raw, cleaned, "email").is_ok());
    }

    #[test]
    fn bullet_boilerplate_symbols_are_whitelisted() {
        let guard = GroundingGuard::default();
        let raw = "Alpha. Beta. Gamma.";
        let cleaned = "• Alpha.\n- Beta.\n* Gamma.";
        assert!(guard.check_grounding(raw, cleaned, "bullet").is_ok());
    }

    #[test]
    fn grounding_violation_display_is_sentence_case() {
        let length_err = GroundingViolation::LengthExceeded {
            raw_len: 10,
            cleaned_len: 100,
            max_allowed: 50,
        };
        assert!(length_err.to_string().starts_with("Cleaned text length exceeds threshold"));

        let novel_err = GroundingViolation::NovelWordsExceeded {
            novel_ratio: 0.6,
            max_ratio: 0.35,
            novel_words: vec!["magic".into()],
        };
        assert!(novel_err.to_string().starts_with("Novel word ratio exceeds threshold"));
    }

    #[test]
    fn safe_unicode_and_filler_window_matching_removes_fillers_and_collapses_punctuation() {
        // Multi-word fillers with safe Unicode
        let raw = "Grüße, you know, die Welt";
        let res = clean_text(raw);
        assert_eq!(res, "Grüße, die Welt.");

        // Leading filler
        let leading = "You know, we should test this";
        let res_leading = clean_text(leading);
        assert_eq!(res_leading, "We should test this.");

        // Trailing filler and period handling
        let trailing = "We should go, you know.";
        let res_trailing = clean_text(trailing);
        assert_eq!(res_trailing, "We should go.");

        // Multiple fillers and collapsing adjacent commas
        let multi = "we should, you know , basically talk";
        let res_multi = clean_text(multi);
        assert_eq!(res_multi, "We should, talk.");

        // "i mean" multi-word filler
        let imean = "I mean, we are ready";
        let res_imean = clean_text(imean);
        assert_eq!(res_imean, "We are ready.");
    }

    #[test]
    fn standardize_sentence_period_guard_prevents_capitalizing_domains_and_extensions() {
        let domain = "Visit narad.org for updates.";
        assert_eq!(clean_text(domain), "Visit narad.org for updates.");

        let file_ext = "Edit main.rs and lib.rs carefully.";
        assert_eq!(clean_text(file_ext), "Edit main.rs and lib.rs carefully.");

        let sentence = "First sentence. Second sentence.";
        assert_eq!(clean_text(sentence), "First sentence. Second sentence.");
    }

    #[test]
    fn email_greeting_and_signoff_refinements() {
        // Greeting without comma
        let greeting_raw = "hello team the meeting is rescheduled";
        let res_greeting = email_text(greeting_raw);
        assert!(res_greeting.starts_with("Hello team the meeting is rescheduled"));
        assert!(!res_greeting.starts_with("Hi,\n\nHello"));
        assert!(res_greeting.ends_with("\n\nBest,\nSender"));

        // Signoff with cheers
        let signoff_raw = "The meeting is rescheduled. Cheers.";
        let res_signoff = email_text(signoff_raw);
        assert_eq!(res_signoff, "Hi,\n\nThe meeting is rescheduled. Cheers.");

        // Both greeting and regards
        let both_raw = "Dear Alice, the meeting is at two. Best regards.";
        let res_both = email_text(both_raw);
        assert_eq!(res_both, "Dear Alice, the meeting is at two. Best regards.");
    }

    #[test]
    fn standardize_sentence_closing_quotes_and_brackets() {
        let quotes = "He said, \"hello.\" we agreed.";
        assert_eq!(clean_text(quotes), "He said, \"hello.\" We agreed.");

        let parens = "He noted (see docs.) we proceeded.";
        assert_eq!(clean_text(parens), "He noted (see docs.) We proceeded.");

        let single_quote = "He said, 'done.' next step.";
        assert_eq!(clean_text(single_quote), "He said, 'done.' Next step.");

        let brackets = "Reference [see item.] next item.";
        assert_eq!(clean_text(brackets), "Reference [see item.] Next item.");
    }

    #[test]
    fn email_signoff_accepts_personal_name_on_final_line() {
        let two_token = "The report is ready.\n\nBest regards,\nJane Doe";
        let res_two = email_text(two_token);
        assert_eq!(res_two, "Hi,\n\nThe report is ready.\n\nBest regards,\nJane Doe");

        let single_token = "The report is ready.\n\nCheers,\nAlex";
        let res_single = email_text(single_token);
        assert_eq!(res_single, "Hi,\n\nThe report is ready.\n\nCheers,\nAlex");

        let with_greeting = "Hi team,\n\nThe report is ready.\n\nSincerely,\nAlice Smith";
        let res_with_greeting = email_text(with_greeting);
        assert_eq!(res_with_greeting, "Hi team,\n\nThe report is ready.\n\nSincerely,\nAlice Smith");
    }

    #[test]
    fn extract_spoken_mode_prefix_recognizes_all_modes_and_variations() {
        assert_eq!(
            extract_spoken_mode_prefix("plain: hello world"),
            Some(("plain", "hello world"))
        );
        assert_eq!(
            extract_spoken_mode_prefix("plain hello world"),
            Some(("plain", "hello world"))
        );
        assert_eq!(
            extract_spoken_mode_prefix("clean: don't do that"),
            Some(("clean", "don't do that"))
        );
        assert_eq!(
            extract_spoken_mode_prefix("clean don't do that"),
            Some(("clean", "don't do that"))
        );
        assert_eq!(
            extract_spoken_mode_prefix("email: meeting at two"),
            Some(("email", "meeting at two"))
        );
        assert_eq!(
            extract_spoken_mode_prefix("email meeting at two"),
            Some(("email", "meeting at two"))
        );
        assert_eq!(
            extract_spoken_mode_prefix("bullet: review the pr"),
            Some(("bullet", "review the pr"))
        );
        assert_eq!(
            extract_spoken_mode_prefix("bullet review the pr"),
            Some(("bullet", "review the pr"))
        );
        assert_eq!(
            extract_spoken_mode_prefix("bullet list: step one"),
            Some(("bullet", "step one"))
        );
        assert_eq!(
            extract_spoken_mode_prefix("bullet list step one"),
            Some(("bullet", "step one"))
        );
        assert_eq!(
            extract_spoken_mode_prefix("bullet points: item a"),
            Some(("bullet", "item a"))
        );
        assert_eq!(
            extract_spoken_mode_prefix("bullet points item a"),
            Some(("bullet", "item a"))
        );
        assert_eq!(
            extract_spoken_mode_prefix("code: fn main() {}"),
            Some(("code", "fn main() {}"))
        );
        assert_eq!(
            extract_spoken_mode_prefix("code fn main() {}"),
            Some(("code", "fn main() {}"))
        );
    }

    #[test]
    fn extract_spoken_mode_prefix_case_insensitivity_and_punctuation_trimming() {
        assert_eq!(
            extract_spoken_mode_prefix("EMAIL: Hello world"),
            Some(("email", "Hello world"))
        );
        assert_eq!(
            extract_spoken_mode_prefix("Bullet List: Alpha"),
            Some(("bullet", "Alpha"))
        );
        assert_eq!(
            extract_spoken_mode_prefix("email: - hello world."),
            Some(("email", "hello world"))
        );
        assert_eq!(
            extract_spoken_mode_prefix("  clean:  , don't do that!  "),
            Some(("clean", "don't do that"))
        );
        assert_eq!(extract_spoken_mode_prefix("just normal dictation"), None);
        assert_eq!(extract_spoken_mode_prefix("plaintext without space"), None);
        assert_eq!(extract_spoken_mode_prefix("emailing someone"), None);
    }

    #[test]
    fn apply_selection_command_transforms() {
        // Formal / Professional
        let formal = apply_selection_command(
            "don't do it, won't work, can't stay",
            "make this formal",
        );
        assert_eq!(formal, "Do not do it, will not work, cannot stay.");
        let prof = apply_selection_command("it's ready", "professional tone");
        assert_eq!(prof, "It is ready.");

        // Shorten / Concise / Summarize
        let concise = apply_selection_command(
            "In order to proceed, we basically really need to test this, you know.",
            "shorten this",
        );
        assert_eq!(concise, "To proceed, we need to test this.");
        let concise2 = apply_selection_command(
            "Due to the fact that it is broken, we must fix it.",
            "make it concise",
        );
        assert_eq!(concise2, "Because it is broken, we must fix it.");

        // Bullets / List
        let bullets = apply_selection_command(
            "Review code. Run tests. Deploy release.",
            "make a bullet list",
        );
        assert_eq!(
            bullets,
            "• Review code.\n• Run tests.\n• Deploy release."
        );

        // Code / Syntax
        let code_block = apply_selection_command(
            "fn main() {\n    println!(\"hello\");\n}",
            "format as code",
        );
        assert_eq!(
            code_block,
            "```\nfn main() {\n    println!(\"hello\");\n}\n```"
        );
        let code_inline = apply_selection_command("let x = 42;", "format code");
        assert_eq!(code_inline, "`let x = 42;`");
        let code_single_block = apply_selection_command("let x = 42;", "format code block");
        assert_eq!(code_single_block, "```\nlet x = 42;\n```");

        // Email
        let email = apply_selection_command(
            "The project is on track for Friday.",
            "format as email",
        );
        assert_eq!(
            email,
            "Hi,\n\nThe project is on track for Friday.\n\nBest,\nSender"
        );

        // Uppercase / All caps
        assert_eq!(
            apply_selection_command("hello world", "uppercase"),
            "HELLO WORLD"
        );
        assert_eq!(
            apply_selection_command("important notice", "make this all caps"),
            "IMPORTANT NOTICE"
        );

        // Lowercase
        assert_eq!(
            apply_selection_command("HELLO World", "lowercase"),
            "hello world"
        );

        // Title case / Capitalized
        assert_eq!(
            apply_selection_command("quick brown fox", "title case"),
            "Quick Brown Fox"
        );
        assert_eq!(
            apply_selection_command("user management service", "capitalized"),
            "User Management Service"
        );

        // Clean / Grammar / Fix
        let clean = apply_selection_command(
            "um we we need to uh test this you know",
            "fix grammar",
        );
        assert_eq!(clean, "We need to test this.");

        // Default fallback
        let fallback = apply_selection_command(
            "um basically hello world",
            "do something else",
        );
        assert_eq!(fallback, "Hello world.");
    }

    #[test]
    fn selection_command_avoids_false_positives_and_supports_synonyms() {
        // "listen" must not trigger bullet list
        let not_list = apply_selection_command("hello world", "listen to this sentence");
        assert_ne!(not_list, "• Hello world");
        assert_eq!(not_list, "Hello world.");

        // "informal" must not trigger format_formal
        let informal = apply_selection_command("don't do it", "make this informal");
        assert_ne!(informal, "Do not do it.");

        // "lower case" synonym
        let lower = apply_selection_command("HELLO WORLD", "make this lower case");
        assert_eq!(lower, "hello world");

        // "capitalize" synonym
        let cap = apply_selection_command("hello world", "capitalize this");
        assert_eq!(cap, "Hello World");
    }

    #[test]
    fn selection_command_preserves_newlines_and_paragraphs() {
        let input = "First paragraph can't wait.\n\nSecond paragraph won't wait.";
        let formal = apply_selection_command(input, "formal tone");
        assert_eq!(
            formal,
            "First paragraph cannot wait.\n\nSecond paragraph will not wait."
        );

        let concise = apply_selection_command(
            "At the present time, we basically agree.\n\nIn order to proceed, we must test.",
            "make concise",
        );
        assert_eq!(
            concise,
            "Now, we agree.\n\nTo proceed, we must test."
        );
    }

    #[test]
    fn code_spoken_prefix_preserves_trailing_semicolon() {
        assert_eq!(
            extract_spoken_mode_prefix("code: let x = 42;"),
            Some(("code", "let x = 42;"))
        );
    }
}


