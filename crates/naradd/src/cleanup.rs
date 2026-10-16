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

/// Strip filler phrases and filler words from raw text using safe token window matching.
fn strip_fillers(raw: &str) -> String {
    let tokens: Vec<&str> = raw.split_whitespace().collect();
    if tokens.is_empty() {
        return String::new();
    }

    let word_fillers = ["um", "uh", "er", "ah", "basically"];
    let mut filtered_tokens: Vec<&str> = Vec::with_capacity(tokens.len());
    let mut i = 0;
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
                i += 2;
                continue;
            }
        }

        // Check 1-token filler
        if word_fillers.contains(&core0.as_str()) {
            i += 1;
            continue;
        }

        filtered_tokens.push(tokens[i]);
        i += 1;
    }

    let joined = filtered_tokens.join(" ");
    collapse_orphaned_punctuation(&joined)
}

/// Remove consecutive duplicate words.
fn remove_repetitions(text: &str) -> String {
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
                if i + 1 == char_vec.len() || char_vec[i + 1].is_whitespace() {
                    cap_next = true;
                }
            } else if c == '!' || c == '?' {
                cap_next = true;
            } else if !c.is_whitespace() {
                cap_next = false;
            }
            final_text.push(c);
        }
    }

    if !final_text.ends_with('.') && !final_text.ends_with('!') && !final_text.ends_with('?') {
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
    let body = clean_text(raw);
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

    let signoff_keywords = ["best", "regards", "sincerely", "cheers"];
    let lines: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
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
        } else if last_tokens.as_slice() == ["sender"] {
            if let Some(&prev_line) = lines.iter().rev().nth(1) {
                let prev_tokens: Vec<String> = prev_line
                    .split_whitespace()
                    .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
                    .filter(|w| !w.is_empty())
                    .collect();
                prev_tokens
                    .iter()
                    .rev()
                    .take(3)
                    .any(|w| signoff_keywords.contains(&w.as_str()))
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    match (has_greeting, has_signoff) {
        (true, true) => body,
        (true, false) => format!("{body}\n\nBest,\nSender"),
        (false, true) => format!("Hi,\n\n{body}"),
        (false, false) => format!("Hi,\n\n{body}\n\nBest,\nSender"),
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
}


