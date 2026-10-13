//! Spoken commands, then replacements (FR-TXT-1, FR-TXT-3).
//!
//! Run after the hallucination filter. Commands are taken from the raw words
//! first, so a replacement cannot override "new line". Written text is not
//! scanned for commands or for further phrases.

/// Apply case-insensitive whole-phrase commands. Output keeps other words' case.
pub fn apply_commands(raw: &str) -> String {
    render(&parse_events(raw))
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
    Scratch,
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
        if let Some((command, width)) = command_at(&tokens, i) {
            events.push(match command {
                Command::NewLine => Event::NewLine,
                Command::NewParagraph => Event::NewParagraph,
                Command::Period => Event::Period,
                Command::Comma => Event::Comma,
                Command::Question => Event::Question,
                Command::Scratch => Event::Scratch,
            });
            i += width;
        } else {
            events.push(Event::Word(tokens[i].to_string()));
            i += 1;
        }
    }
    events
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
            Event::Scratch => out.apply(Command::Scratch),
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
    Scratch,
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
        ("period", _) => Some((Command::Period, 1)),
        ("comma", _) => Some((Command::Comma, 1)),
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
}
