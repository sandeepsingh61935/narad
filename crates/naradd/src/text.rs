//! Spoken commands (FR-TXT-1).
//!
//! Run after the hallucination filter. Later replacements must not see these
//! phrases: a snippet cannot override "new line".

/// Apply case-insensitive whole-phrase commands. Output keeps other words' case.
pub fn apply_commands(raw: &str) -> String {
    let tokens: Vec<&str> = raw.split_whitespace().collect();
    let mut out = Output::default();
    let mut i = 0;
    while i < tokens.len() {
        if let Some((command, width)) = command_at(&tokens, i) {
            out.apply(command);
            i += width;
        } else {
            out.push_word(tokens[i]);
            i += 1;
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
}
