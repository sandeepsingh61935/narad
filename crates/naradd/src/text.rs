//! Spoken commands, then replacements, then vocabulary (FR-TXT-1..4).
//!
//! Run after the hallucination filter. Commands are taken from the raw words
//! first, so a replacement cannot override "new line". Written text is not
//! scanned for commands or for further phrases.

/// Apply case-insensitive whole-phrase commands. Output keeps other words' case.
pub fn apply_commands(raw: &str) -> String {
    render(&parse_events(raw))
}

/// Apply commands and format punctuation for the given mode (e.g. "code").
pub fn apply_commands_in_mode(raw: &str, mode: &str) -> String {
    let rendered = render(&parse_events(raw));
    format_mode_punctuation(&rendered, mode)
}

/// One spoken phrase and the literal text that replaces it. No built-in pairs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replacement {
    pub spoken: String,
    pub written: String,
}

/// Longest-match replacements on an already command-free string.
pub fn apply_replacements(text: &str, pairs: &[Replacement]) -> String {
    if !pairs.iter().any(|pair| pair.spoken.split_whitespace().next().is_some()) {
        return text.to_string();
    }
    render(&replace_words(parse_literals(text), pairs))
}

/// Commands, then replacements. Replacement output is not scanned again.
pub fn apply_commands_then_replacements(raw: &str, pairs: &[Replacement]) -> String {
    render(&replace_words(parse_events(raw), pairs))
}

/// Commands, then replacements, then vocabulary. Neither table scans written text.
pub fn apply_commands_replacements_then_vocabulary(
    raw: &str,
    replacements: &[Replacement],
    vocabulary: &[Replacement],
) -> String {
    let after_commands = parse_events(raw);
    let after_replacements = replace_words(after_commands, replacements);
    render(&replace_words(after_replacements, vocabulary))
}

/// Determines if candidate punctuation word appears in a literal context
/// (e.g. "colon cancer", "colonial period", "100 meter dash") based on
/// 3-token left and right syntactic lookaround.
pub fn is_literal_punctuation_context(left: &[&str], punct_word: &str, right: &[&str]) -> bool {
    let _ = punct_word;

    // 1. Right Context: Anatomical and compound nouns force literal word representation.
    if let Some(next) = right.first() {
        let next_clean = next
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_ascii_lowercase();
        match next_clean.as_str() {
            "cancer" | "cancers" | "tumor" | "tumors" | "surgery" | "surgeries" | "polyps"
            | "polyp" | "screening" | "screenings" | "drama" | "dramas" | "biopsy"
            | "resection" | "examination" | "exam" => {
                return true;
            }
            _ => {}
        }
    }

    // 2. Left Context: Determiners, Prepositions, and Compound Modifiers force literal word representation.
    if let Some(prev) = left.last() {
        let prev_clean = prev
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_ascii_lowercase();
        match prev_clean.as_str() {
            // Determiners
            "the" | "a" | "an" | "this" | "that" | "my" | "your" | "his" | "her" | "their"
            | "our" | "its" => {
                return true;
            }
            // Prepositions
            "of" | "in" | "into" | "through" | "with" | "by" | "for" | "at" | "from" | "on"
            | "to" | "about" | "between" => {
                return true;
            }
            // Compound Modifiers
            "colonial" | "grace" | "meter" | "meters" | "100-meter" => {
                return true;
            }
            _ => {}
        }

        let prev_lower = prev.to_ascii_lowercase();
        if prev_lower.ends_with("100 meter") || prev_lower.ends_with("100 meters") {
            return true;
        }
    }

    // Multi-token Compound Modifier: "100 meter" as separate tokens
    if left.len() >= 2 {
        let prev2 = left[left.len() - 2]
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_ascii_lowercase();
        let prev1 = left[left.len() - 1]
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_ascii_lowercase();
        if prev2 == "100" && (prev1 == "meter" || prev1 == "meters") {
            return true;
        }
    }

    false
}

/// Disambiguates homophones (e.g. they're/their/there and cache/cash) using local POS
/// and collocation heuristics.
pub fn disambiguate_homophones(words: &[&str]) -> Vec<String> {
    let mut result = Vec::with_capacity(words.len());

    for (i, &word) in words.iter().enumerate() {
        let (prefix, core, suffix) = split_affixes(word);
        let core_lower = core.to_ascii_lowercase();

        // 1. they're / their / there / theyre
        if matches!(core_lower.as_str(), "they're" | "their" | "there" | "theyre") {
            let next_word = words.get(i + 1).map(|w| {
                let (_, next_core, _) = split_affixes(w);
                next_core.to_ascii_lowercase()
            });

            let chosen = if let Some(ref next) = next_word {
                if is_theyre_trigger(next) {
                    "they're"
                } else if is_their_trigger(next) {
                    "their"
                } else {
                    "there"
                }
            } else {
                "there"
            };

            let formatted = match_case(core, chosen);
            result.push(format!("{}{}{}", prefix, formatted, suffix));
            continue;
        }

        // 2. cache / cash / caches / cashes
        if matches!(core_lower.as_str(), "cache" | "cash" | "caches" | "cashes") {
            let is_plural = core_lower == "caches" || core_lower == "cashes";
            let window_start = i.saturating_sub(3);
            let window_end = (i + 4).min(words.len());

            let mut has_storage = false;
            let mut has_monetary = false;

            for (idx, &w) in words[window_start..window_end].iter().enumerate() {
                let actual_idx = window_start + idx;
                if actual_idx == i {
                    continue;
                }
                if is_storage_token(w) {
                    has_storage = true;
                }
                if is_monetary_token(w) {
                    has_monetary = true;
                }
            }

            let chosen = if has_storage {
                if is_plural {
                    "caches"
                } else {
                    "cache"
                }
            } else if has_monetary {
                if is_plural {
                    "cashes"
                } else {
                    "cash"
                }
            } else {
                core
            };

            let formatted = match_case(core, chosen);
            result.push(format!("{}{}{}", prefix, formatted, suffix));
            continue;
        }

        result.push(word.to_string());
    }

    result
}

fn split_affixes(s: &str) -> (&str, &str, &str) {
    let start = s.find(|c: char| c.is_alphanumeric()).unwrap_or(0);
    let end = s.rfind(|c: char| c.is_alphanumeric()).map_or(s.len(), |idx| {
        idx + s[idx..].chars().next().map_or(0, |c| c.len_utf8())
    });
    if start >= end {
        ("", s, "")
    } else {
        (&s[..start], &s[start..end], &s[end..])
    }
}

fn match_case(template: &str, replacement: &str) -> String {
    let mut alpha_chars = template.chars().filter(|c| c.is_alphabetic());
    let first = alpha_chars.next();
    let second = alpha_chars.next();
    match (first, second) {
        (Some(f), Some(s)) if f.is_uppercase() && s.is_uppercase() => {
            replacement.to_ascii_uppercase()
        }
        (Some(f), _) if f.is_uppercase() => {
            let mut res = String::new();
            let mut rep_chars = replacement.chars();
            if let Some(rf) = rep_chars.next() {
                res.extend(rf.to_uppercase());
            }
            res.extend(rep_chars);
            res
        }
        _ => replacement.to_ascii_lowercase(),
    }
}

fn is_theyre_trigger(next: &str) -> bool {
    matches!(
        next,
        "coming"
            | "going"
            | "here"
            | "ready"
            | "planning"
            | "trying"
            | "doing"
            | "making"
            | "working"
            | "waiting"
            | "running"
            | "looking"
            | "leaving"
            | "arriving"
            | "staying"
            | "saying"
            | "talking"
            | "moving"
            | "eating"
            | "thinking"
            | "calling"
            | "asking"
            | "playing"
            | "supposed"
            | "able"
            | "not"
            | "all"
            | "both"
            | "still"
            | "always"
            | "already"
            | "gonna"
            | "welcome"
            | "back"
            | "gone"
            | "done"
            | "finished"
            | "right"
            | "tired"
            | "happy"
    )
}

fn is_their_trigger(next: &str) -> bool {
    matches!(
        next,
        "car"
            | "cars"
            | "house"
            | "houses"
            | "code"
            | "codes"
            | "idea"
            | "ideas"
            | "family"
            | "families"
            | "room"
            | "rooms"
            | "home"
            | "homes"
            | "work"
            | "friends"
            | "friend"
            | "children"
            | "child"
            | "kids"
            | "kid"
            | "parents"
            | "parent"
            | "team"
            | "teams"
            | "way"
            | "ways"
            | "name"
            | "names"
            | "dog"
            | "dogs"
            | "cat"
            | "cats"
            | "place"
            | "places"
            | "money"
            | "computer"
            | "computers"
            | "phone"
            | "phones"
            | "life"
            | "lives"
            | "time"
            | "times"
            | "job"
            | "jobs"
            | "data"
            | "project"
            | "projects"
            | "hands"
            | "hand"
            | "faces"
            | "face"
            | "eyes"
            | "eye"
            | "own"
            | "first"
            | "new"
            | "old"
            | "best"
            | "favorite"
            | "bag"
            | "bags"
    )
}

fn is_storage_token(w: &str) -> bool {
    let lower = w.trim_matches(|c: char| !c.is_alphanumeric()).to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "clear"
            | "cleared"
            | "clearing"
            | "memory"
            | "memories"
            | "cpu"
            | "gpu"
            | "disk"
            | "disks"
            | "l1"
            | "l2"
            | "l3"
            | "redis"
            | "memcached"
            | "ram"
            | "storage"
            | "ssd"
            | "buffer"
            | "buffers"
            | "hit"
            | "hits"
            | "miss"
            | "misses"
            | "flush"
            | "flushed"
            | "flushing"
            | "invalidate"
            | "invalidation"
            | "server"
            | "browser"
    )
}

fn is_monetary_token(w: &str) -> bool {
    let lower = w.trim_matches(|c: char| !c.is_alphanumeric()).to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "pay"
            | "pays"
            | "paying"
            | "paid"
            | "payment"
            | "payments"
            | "money"
            | "bills"
            | "bill"
            | "register"
            | "registers"
            | "cold"
            | "hard"
            | "dollar"
            | "dollars"
            | "cent"
            | "cents"
            | "petty"
            | "flow"
            | "atm"
            | "spend"
            | "spent"
            | "cost"
            | "costs"
            | "price"
            | "prices"
            | "bank"
            | "check"
            | "wallet"
            | "credit"
            | "debit"
    )
}

/// Formats punctuation spacing according to the specified editing mode.
/// In "code" mode:
/// - Double colons `::` are formatted without inner whitespace (`std::collections`).
/// - Colons `:`, semicolons `;`, and dots `.` attach directly to the preceding word without whitespace.
pub fn format_mode_punctuation(text: &str, mode: &str) -> String {
    if !mode.eq_ignore_ascii_case("code") {
        return text.to_string();
    }

    // Step 1: Normalize double colons.
    // Replace any sequence of ':' followed by optional horizontal whitespace and ':' with '::'.
    // Remove horizontal whitespace immediately before and after the double colon.
    let mut step1 = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ':' {
            let mut j = i + 1;
            while j < chars.len() && (chars[j] == ' ' || chars[j] == '\t') {
                j += 1;
            }
            if j < chars.len() && chars[j] == ':' {
                // Found double colon
                while step1.ends_with([' ', '\t']) {
                    step1.pop();
                }
                step1.push_str("::");
                i = j + 1;
                while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t') {
                    i += 1;
                }
                continue;
            }
        }
        step1.push(chars[i]);
        i += 1;
    }

    // Step 2: Attach colons (:), semicolons (;), and dots (.) directly to preceding word without spaces.
    let mut out = String::with_capacity(step1.len());
    let chars1: Vec<char> = step1.chars().collect();
    let mut k = 0;
    while k < chars1.len() {
        let c = chars1[k];
        if c == ':' || c == ';' || c == '.' {
            let is_double_colon = (c == ':' && k + 1 < chars1.len() && chars1[k + 1] == ':')
                || (c == ':' && k > 0 && chars1[k - 1] == ':');

            if !is_double_colon {
                while out.ends_with([' ', '\t']) {
                    out.pop();
                }
            }
            out.push(c);
            k += 1;
        } else {
            out.push(c);
            k += 1;
        }
    }

    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    /// A spoken word that later phrase tables may still match.
    Word(String),
    /// Replacement or vocabulary output. Later tables must not scan it.
    Literal(String),
    NewLine,
    NewParagraph,
    Period,
    Comma,
    Question,
    Colon,
    Semicolon,
    Dash,
    EmDash,
    Scratch,
    /// A case phrase ended the previous segment. A bare phrase adds nothing.
    Boundary,
}

fn parse_literals(text: &str) -> Vec<Event> {
    text.split_whitespace()
        .map(|word| Event::Word(word.to_string()))
        .collect()
}

fn parse_events(raw: &str) -> Vec<Event> {
    let tokens: Vec<&str> = raw.split_whitespace().collect();
    let mut events = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if let Some(style) = case_style_at(&tokens, i) {
            i += 2;
            let start = i;
            while i < tokens.len()
                && command_at(&tokens, i).is_none()
                && case_style_at(&tokens, i).is_none()
            {
                i += 1;
            }
            events.push(Event::Boundary);
            if i > start {
                events.push(Event::Word(restyle(&tokens[start..i], style)));
            }
            continue;
        }

        // Eager escape: if preceded by "literal" or "word", suppress command conversion
        // and emit the command words literally.
        let lower = tokens[i].to_ascii_lowercase();
        if (lower == "literal" || lower == "word") && i + 1 < tokens.len() {
            if let Some((_cmd, width)) = command_at(&tokens, i + 1) {
                for k in 0..width {
                    events.push(Event::Word(tokens[i + 1 + k].to_string()));
                }
                i += 1 + width;
                continue;
            }
        }

        if let Some((command, width)) = command_at(&tokens, i) {
            if is_punctuation_command(&command) {
                let left = &tokens[..i];
                let right = &tokens[i + width..];
                if is_literal_punctuation_context(left, tokens[i], right) {
                    for k in 0..width {
                        events.push(Event::Word(tokens[i + k].to_string()));
                    }
                    i += width;
                    continue;
                }
            }

            events.push(match command {
                Command::NewLine => Event::NewLine,
                Command::NewParagraph => Event::NewParagraph,
                Command::Period => Event::Period,
                Command::Comma => Event::Comma,
                Command::Question => Event::Question,
                Command::Colon => Event::Colon,
                Command::Semicolon => Event::Semicolon,
                Command::Dash => Event::Dash,
                Command::EmDash => Event::EmDash,
                Command::Scratch => Event::Scratch,
            });
            i += width;
        } else {
            events.push(Event::Word(tokens[i].to_string()));
            i += 1;
        }
    }

    disambiguate_event_words(&mut events);
    events
}

fn disambiguate_event_words(events: &mut [Event]) {
    let mut i = 0;
    while i < events.len() {
        if !matches!(events[i], Event::Word(_)) {
            i += 1;
            continue;
        }
        let start = i;
        while i < events.len() && matches!(events[i], Event::Word(_)) {
            i += 1;
        }
        let end = i;
        let words: Vec<&str> = events[start..end]
            .iter()
            .map(|e| match e {
                Event::Word(w) => w.as_str(),
                _ => unreachable!(),
            })
            .collect();
        let disambiguated = disambiguate_homophones(&words);
        for (event, new_word) in events[start..end].iter_mut().zip(disambiguated) {
            *event = Event::Word(new_word);
        }
    }
}

enum CaseStyle {
    Camel,
    Snake,
    Kebab,
    Pascal,
    Constant,
}

fn case_style_at(tokens: &[&str], index: usize) -> Option<CaseStyle> {
    let word = tokens[index].to_ascii_lowercase();
    let next = tokens.get(index + 1)?.to_ascii_lowercase();
    if next != "case" {
        return None;
    }
    match word.as_str() {
        "camel" => Some(CaseStyle::Camel),
        "snake" => Some(CaseStyle::Snake),
        "kebab" => Some(CaseStyle::Kebab),
        "pascal" => Some(CaseStyle::Pascal),
        "constant" => Some(CaseStyle::Constant),
        _ => None,
    }
}

fn restyle(words: &[&str], style: CaseStyle) -> String {
    let words: Vec<String> = words.iter().map(|word| word.to_lowercase()).collect();
    match style {
        CaseStyle::Camel => join_hump(&words, false),
        CaseStyle::Pascal => join_hump(&words, true),
        CaseStyle::Snake => words.join("_"),
        CaseStyle::Kebab => words.join("-"),
        CaseStyle::Constant => words
            .iter()
            .map(|word| word.to_uppercase())
            .collect::<Vec<_>>()
            .join("_"),
    }
}

fn join_hump(words: &[String], capitalize_first: bool) -> String {
    let mut text = String::new();
    for (index, word) in words.iter().enumerate() {
        let capitalize = index > 0 || capitalize_first;
        if !capitalize {
            text.push_str(word);
            continue;
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            text.extend(first.to_uppercase());
            text.push_str(chars.as_str());
        }
    }
    text
}

fn replace_words(mut events: Vec<Event>, pairs: &[Replacement]) -> Vec<Event> {
    let phrases: Vec<Phrase> = pairs
        .iter()
        .filter_map(|pair| {
            let spoken: Vec<String> = pair
                .spoken
                .split_whitespace()
                .map(|word| word.to_ascii_lowercase())
                .collect();
            if spoken.is_empty() {
                None
            } else {
                Some(Phrase {
                    spoken,
                    written: pair.written.clone(),
                })
            }
        })
        .collect();
    if phrases.is_empty() {
        return events;
    }

    let mut index = 0;
    while index < events.len() {
        if !matches!(events[index], Event::Word(_)) {
            index += 1;
            continue;
        }
        let Some((width, written)) = longest_phrase(&events, index, &phrases) else {
            index += 1;
            continue;
        };
        events.splice(
            index..index + width,
            std::iter::once(Event::Literal(written)),
        );
        index += 1;
    }
    events.retain(|event| match event {
        Event::Word(text) | Event::Literal(text) => !text.is_empty(),
        _ => true,
    });
    events
}

struct Phrase {
    spoken: Vec<String>,
    written: String,
}

/// Longest spoken phrase at `index`. Equal lengths keep the earlier pair.
fn longest_phrase(events: &[Event], index: usize, phrases: &[Phrase]) -> Option<(usize, String)> {
    let mut best: Option<(usize, usize, String)> = None;
    for (order, phrase) in phrases.iter().enumerate() {
        let width = phrase.spoken.len();
        if index + width > events.len() {
            continue;
        }
        let matches = phrase.spoken.iter().enumerate().all(|(offset, spoken)| {
            match &events[index + offset] {
                Event::Word(word) => word.to_ascii_lowercase() == *spoken,
                _ => false,
            }
        });
        if !matches {
            continue;
        }
        let replace = match &best {
            None => true,
            Some((best_width, best_order, _)) => {
                width > *best_width || (width == *best_width && order < *best_order)
            }
        };
        if replace {
            best = Some((width, order, phrase.written.clone()));
        }
    }
    best.map(|(width, _, written)| (width, written))
}

fn render(events: &[Event]) -> String {
    let mut out = Output::default();
    for event in events {
        match event {
            Event::Word(word) | Event::Literal(word) => out.push_word(word),
            Event::NewLine => out.apply(Command::NewLine),
            Event::NewParagraph => out.apply(Command::NewParagraph),
            Event::Period => out.apply(Command::Period),
            Event::Comma => out.apply(Command::Comma),
            Event::Question => out.apply(Command::Question),
            Event::Colon => out.apply(Command::Colon),
            Event::Semicolon => out.apply(Command::Semicolon),
            Event::Dash => out.apply(Command::Dash),
            Event::EmDash => out.apply(Command::EmDash),
            Event::Scratch => out.apply(Command::Scratch),
            Event::Boundary => out.close(""),
        }
    }
    out.render()
}

enum Command {
    NewLine,
    NewParagraph,
    Period,
    Comma,
    Question,
    Colon,
    Semicolon,
    Dash,
    EmDash,
    Scratch,
}

fn is_punctuation_command(command: &Command) -> bool {
    matches!(
        command,
        Command::Period
            | Command::Comma
            | Command::Question
            | Command::Colon
            | Command::Semicolon
            | Command::Dash
            | Command::EmDash
    )
}

fn command_at(tokens: &[&str], index: usize) -> Option<(Command, usize)> {
    let word = tokens[index].to_ascii_lowercase();
    let next = tokens
        .get(index + 1)
        .map(|token| token.to_ascii_lowercase());
    match (word.as_str(), next.as_deref()) {
        ("new", Some("line")) => Some((Command::NewLine, 2)),
        ("new", Some("paragraph")) => Some((Command::NewParagraph, 2)),
        ("question", Some("mark")) => Some((Command::Question, 2)),
        ("scratch", Some("that")) => Some((Command::Scratch, 2)),
        ("em", Some("dash")) => Some((Command::EmDash, 2)),
        ("long", Some("dash")) => Some((Command::EmDash, 2)),
        ("em-dash", _) => Some((Command::EmDash, 1)),
        ("long-dash", _) => Some((Command::EmDash, 1)),
        ("period", _) => Some((Command::Period, 1)),
        ("comma", _) => Some((Command::Comma, 1)),
        ("colon", _) => Some((Command::Colon, 1)),
        ("semicolon", _) => Some((Command::Semicolon, 1)),
        ("dash", _) => Some((Command::Dash, 1)),
        ("hyphen", _) => Some((Command::Dash, 1)),
        _ => None,
    }
}

#[derive(Default)]
struct Output {
    /// Closed pieces. A command ends the open piece.
    segments: Vec<String>,
    /// Words spoken since the previous command.
    open: String,
    /// The previous action was scratch, so another scratch pops one segment.
    scratch_again: bool,
}

impl Output {
    fn push_word(&mut self, word: &str) {
        if !self.open.is_empty() {
            self.open.push(' ');
        }
        self.open.push_str(word);
        self.scratch_again = false;
    }

    fn apply(&mut self, command: Command) {
        match command {
            Command::NewLine => self.close("\n"),
            Command::NewParagraph => self.close("\n\n"),
            Command::Period => self.punct('.'),
            Command::Comma => self.punct(','),
            Command::Question => self.punct('?'),
            Command::Colon => self.punct(':'),
            Command::Semicolon => self.punct(';'),
            Command::Dash => self.punct('-'),
            Command::EmDash => self.punct('—'),
            Command::Scratch => self.scratch(),
        }
    }

    fn punct(&mut self, mark: char) {
        if self.open.is_empty() {
            // A bare mark is dropped. It is still a command, so the next words
            // start a new segment and a following scratch can pop this boundary.
            self.scratch_again = false;
            return;
        }
        self.open.push(mark);
        self.close("");
    }

    fn close(&mut self, suffix: &str) {
        let mut segment = std::mem::take(&mut self.open);
        segment.push_str(suffix);
        if !segment.is_empty() {
            self.segments.push(segment);
        }
        self.scratch_again = false;
    }

    fn scratch(&mut self) {
        if !self.open.is_empty() {
            self.open.clear();
            self.scratch_again = true;
            return;
        }
        if self.scratch_again {
            self.segments.pop();
        }
        self.scratch_again = true;
    }

    fn render(&self) -> String {
        let mut parts: Vec<&str> = self.segments.iter().map(String::as_str).collect();
        if !self.open.is_empty() {
            parts.push(&self.open);
        }
        let mut text = String::new();
        for part in parts {
            if text.is_empty() {
                text.push_str(part);
                continue;
            }
            if !text.ends_with('\n') {
                text.push(' ');
            }
            text.push_str(part);
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn punctuation_attaches_and_a_bare_mark_is_dropped() {
        assert_eq!(apply_commands("hello period"), "hello.");
        assert_eq!(apply_commands("Hello PERIOD world"), "Hello. world");
        assert_eq!(apply_commands("hello comma world"), "hello, world");
        assert_eq!(apply_commands("hello question mark world"), "hello? world");
        assert_eq!(apply_commands("period hello"), "hello");
        assert_eq!(apply_commands("comma"), "");
        assert_eq!(apply_commands("question mark"), "");
        assert_eq!(apply_commands("hello period period"), "hello.");
    }

    #[test]
    fn line_breaks_are_whole_phrases() {
        assert_eq!(apply_commands("hello new line world"), "hello\nworld");
        assert_eq!(
            apply_commands("hello new paragraph world"),
            "hello\n\nworld"
        );
        assert_eq!(apply_commands("new line"), "\n");
        assert_eq!(apply_commands("NEW PARAGRAPH"), "\n\n");
        assert_eq!(apply_commands("brand newline"), "brand newline");
    }

    #[test]
    fn commands_do_not_match_inside_words() {
        assert_eq!(
            apply_commands("the periodical report"),
            "the periodical report"
        );
        assert_eq!(apply_commands("scratching that"), "scratching that");
        assert_eq!(apply_commands("comma-separated"), "comma-separated");
    }

    #[test]
    fn scratch_removes_the_segment_since_the_previous_command() {
        assert_eq!(apply_commands("scratch that"), "");
        assert_eq!(apply_commands("hello scratch that"), "");
        assert_eq!(apply_commands("hello scratch that world"), "world");
        assert_eq!(apply_commands("world period scratch that"), "world.");
        assert_eq!(
            apply_commands("hello period world scratch that"),
            "hello."
        );
        assert_eq!(
            apply_commands("hello new line world scratch that"),
            "hello\n"
        );
        assert_eq!(apply_commands("world period scratch that scratch that"), "");
        assert_eq!(
            apply_commands("hello scratch that scratch that"),
            ""
        );
        assert_eq!(
            apply_commands("alpha period beta scratch that scratch that gamma"),
            "gamma"
        );
    }

    #[test]
    fn case_phrases_restyle_until_the_next_command() {
        assert_eq!(apply_commands("camel case user name period"), "userName.");
        assert_eq!(apply_commands("snake case user name"), "user_name");
        assert_eq!(apply_commands("kebab case user name"), "user-name");
        assert_eq!(apply_commands("pascal case user name"), "UserName");
        assert_eq!(apply_commands("constant case user name"), "USER_NAME");
        assert_eq!(apply_commands("camel case user 2"), "user2");
        assert_eq!(apply_commands("snake case user 2"), "user_2");
        assert_eq!(apply_commands("hello camel case"), "hello");
        assert_eq!(apply_commands("camel case period"), "");
        assert_eq!(
            apply_commands("camel case user name scratch that"),
            ""
        );
        assert_eq!(
            apply_commands("hello camel case user name scratch that"),
            "hello"
        );
        assert_eq!(apply_commands("hello world"), "hello world");
        assert_eq!(apply_commands("hello period"), "hello.");
        assert_eq!(apply_commands("CAMEL CASE User Name"), "userName");
    }

    fn pair(spoken: &str, written: &str) -> Replacement {
        Replacement {
            spoken: spoken.to_string(),
            written: written.to_string(),
        }
    }

    #[test]
    fn replacements_use_longest_whole_phrase_and_keep_written_text() {
        let pairs = [
            pair("new york", "NY"),
            pair("new york city", "NYC"),
            pair("hi", "A"),
            pair("hi", "B"),
            pair("cat", "fat cat"),
        ];
        assert_eq!(apply_replacements("visit new york city today", &pairs), "visit NYC today");
        assert_eq!(apply_replacements("visit New York today", &pairs), "visit NY today");
        assert_eq!(apply_replacements("Hi", &pairs), "A");
        assert_eq!(apply_replacements("the category", &pairs), "the category");
        assert_eq!(apply_replacements("cat", &pairs), "fat cat");
        assert_eq!(apply_replacements("unchanged words", &[]), "unchanged words");
        assert_eq!(apply_replacements("hello  world", &[]), "hello  world");
    }

    #[test]
    fn replacements_follow_commands_and_are_not_scanned_again() {
        let newline = [pair("new line", "NL")];
        assert_eq!(
            apply_commands_then_replacements("hello new line world", &newline),
            "hello\nworld"
        );
        let expands = [pair("hi", "new line"), pair("see you", "bye")];
        assert_eq!(
            apply_commands_then_replacements("hi there", &expands),
            "new line there"
        );
        assert_eq!(
            apply_commands_then_replacements("see you period", &expands),
            "bye."
        );
        assert_eq!(
            apply_commands_then_replacements("see you scratch that", &expands),
            ""
        );
        let city = [pair("new york city", "NYC"), pair("new york", "NY")];
        assert_eq!(
            apply_commands_then_replacements("new york new line city", &city),
            "NY\ncity"
        );
    }

    #[test]
    fn vocabulary_runs_after_replacements_and_does_not_rewrite_them() {
        let replacements = [pair("big apple", "New York"), pair("ny", "new york")];
        let vocabulary = [pair("new york", "NYC"), pair("york", "York")];
        assert_eq!(
            apply_commands_replacements_then_vocabulary(
                "visit the big apple period",
                &replacements,
                &vocabulary
            ),
            "visit the New York."
        );
        assert_eq!(
            apply_commands_replacements_then_vocabulary("ny", &replacements, &vocabulary),
            "new york"
        );
        assert_eq!(
            apply_commands_replacements_then_vocabulary(
                "new york city",
                &[],
                &[pair("new york city", "NYC"), pair("new york", "NY")]
            ),
            "NYC"
        );
    }

    #[test]
    fn spoken_punctuation_expansion_attaches_and_drops_bare_marks() {
        assert_eq!(apply_commands("hello colon world"), "hello: world");
        assert_eq!(apply_commands("hello semicolon world"), "hello; world");
        assert_eq!(apply_commands("hello dash world"), "hello- world");
        assert_eq!(apply_commands("hello hyphen world"), "hello- world");
        assert_eq!(apply_commands("hello em dash world"), "hello— world");
        assert_eq!(apply_commands("hello long dash world"), "hello— world");

        // Bare marks are dropped
        assert_eq!(apply_commands("colon"), "");
        assert_eq!(apply_commands("semicolon"), "");
        assert_eq!(apply_commands("dash"), "");
        assert_eq!(apply_commands("hyphen"), "");
        assert_eq!(apply_commands("em dash"), "");
        assert_eq!(apply_commands("long dash"), "");
    }

    #[test]
    fn lookaround_protects_literal_punctuation() {
        // Right context anatomical/compound noun
        assert_eq!(apply_commands("colon cancer"), "colon cancer");
        assert_eq!(
            apply_commands("the patient has colon cancer"),
            "the patient has colon cancer"
        );
        assert_eq!(apply_commands("colon surgery"), "colon surgery");
        assert_eq!(apply_commands("colon polyps"), "colon polyps");
        assert_eq!(apply_commands("colon tumor"), "colon tumor");
        assert_eq!(apply_commands("colon screening"), "colon screening");
        assert_eq!(apply_commands("period drama"), "period drama");

        // Left context determiners, prepositions, compound modifiers
        assert_eq!(apply_commands("colonial period"), "colonial period");
        assert_eq!(apply_commands("grace period"), "grace period");
        assert_eq!(apply_commands("the colonial period"), "the colonial period");
        assert_eq!(apply_commands("100 meter dash"), "100 meter dash");

        // Followed by actual command period
        assert_eq!(apply_commands("colon cancer period"), "colon cancer.");
        assert_eq!(
            apply_commands("the colonial period period"),
            "the colonial period."
        );

        // Direct unit checks on is_literal_punctuation_context
        assert!(is_literal_punctuation_context(&[], "colon", &["cancer"]));
        assert!(is_literal_punctuation_context(&["the"], "period", &[]));
        assert!(is_literal_punctuation_context(&["of"], "colon", &[]));
        assert!(is_literal_punctuation_context(&["colonial"], "period", &[]));
        assert!(is_literal_punctuation_context(&["grace"], "period", &[]));
        assert!(is_literal_punctuation_context(&["100", "meter"], "dash", &[]));
        assert!(is_literal_punctuation_context(&["100-meter"], "dash", &[]));
        assert!(!is_literal_punctuation_context(&["hello"], "colon", &["world"]));
        assert!(!is_literal_punctuation_context(&[], "period", &[]));
    }

    #[test]
    fn eager_escape_suppresses_command_conversion() {
        assert_eq!(apply_commands("literal colon"), "colon");
        assert_eq!(apply_commands("word colon"), "colon");
        assert_eq!(apply_commands("literal period"), "period");
        assert_eq!(apply_commands("word comma"), "comma");
        assert_eq!(apply_commands("literal question mark"), "question mark");
        assert_eq!(apply_commands("literal new line"), "new line");
        assert_eq!(apply_commands("literal em dash"), "em dash");
        assert_eq!(apply_commands("literal semicolon"), "semicolon");

        // Normal words "literal" and "word" when not preceding a command
        assert_eq!(apply_commands("literal meaning"), "literal meaning");
        assert_eq!(apply_commands("word of advice"), "word of advice");
        assert_eq!(apply_commands("a single word"), "a single word");
    }

    #[test]
    fn homophone_disambiguation_rules() {
        // they're before verbs, their before nouns, there elsewhere
        assert_eq!(
            apply_commands("they're going to clean their room over there"),
            "they're going to clean their room over there"
        );
        assert_eq!(
            apply_commands("there going to clean there room over their"),
            "they're going to clean their room over there"
        );
        assert_eq!(
            apply_commands("they're coming here because their car is over there"),
            "they're coming here because their car is over there"
        );
        assert_eq!(apply_commands("there is their house"), "there is their house");
        assert_eq!(apply_commands("is there their code"), "is there their code");

        // cache around memory/storage vs cash around monetary
        assert_eq!(apply_commands("clear the cpu cache"), "clear the cpu cache");
        assert_eq!(apply_commands("clear the cpu cash"), "clear the cpu cache");
        assert_eq!(
            apply_commands("pay with cold hard cash"),
            "pay with cold hard cash"
        );
        assert_eq!(
            apply_commands("pay with cold hard cache"),
            "pay with cold hard cash"
        );
        assert_eq!(apply_commands("redis cache miss"), "redis cache miss");
        assert_eq!(apply_commands("register cash bills"), "register cash bills");

        // Direct unit check on disambiguate_homophones
        let input = ["there", "going", "to", "clean", "there", "room", "over", "their"];
        let expected = vec![
            "they're", "going", "to", "clean", "their", "room", "over", "there",
        ];
        assert_eq!(disambiguate_homophones(&input), expected);
    }

    #[test]
    fn code_mode_spacing_and_punctuation_formatting() {
        assert_eq!(
            format_mode_punctuation("std :: vector", "code"),
            "std::vector"
        );
        assert_eq!(
            format_mode_punctuation("std : : collections", "code"),
            "std::collections"
        );
        assert_eq!(
            format_mode_punctuation("std::collections", "code"),
            "std::collections"
        );
        assert_eq!(
            format_mode_punctuation("let x : i32 ;", "code"),
            "let x: i32;"
        );
        assert_eq!(format_mode_punctuation("foo . bar", "code"), "foo. bar");
        assert_eq!(format_mode_punctuation("a :: b :: c", "code"), "a::b::c");

        // Non-code modes are left untouched
        assert_eq!(
            format_mode_punctuation("std :: vector", "plain"),
            "std :: vector"
        );
        assert_eq!(
            format_mode_punctuation("std :: vector", "email"),
            "std :: vector"
        );

        // Integrated apply_commands_in_mode
        assert_eq!(apply_commands_in_mode("std :: vector", "code"), "std::vector");
        assert_eq!(
            apply_commands_in_mode("let x colon i32 semicolon", "code"),
            "let x: i32;"
        );
    }
}
