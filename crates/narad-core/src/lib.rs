//! Platform-agnostic Narad core (pipeline, text, store).

#![forbid(unsafe_code)]

pub mod store;

/// Dictation session state on the hot path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Idle,
    Listening,
    Processing,
    Error,
    Copied,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_state_roundtrip_debug() {
        assert_eq!(format!("{:?}", SessionState::Idle), "Idle");
    }
}
