//! Shared D-Bus contract for `dev.narad.Daemon`.
//!
//! Interface sketch: `docs/ARCHITECTURE.md` §6.
//!
//! ## Phase 1 surface
//!
//! Methods: `Start`, `Stop`, `Cancel`, `Toggle`, `History`, `ReinsertEntry`, `DeleteEntries`,
//! `PasteLast` (FR-INS-4), `UndoLast` (FR-INS-5), `ReportFocus` (FR-CTX-1).
//! Signals: `StateChanged`, `Level`, `EntryAdded`.
//! Properties: `Mode`, `Paused` (writable), `HudVisible` (writable).
//!
//! ## Deferred (Phase 2+)
//!
//! Not defined in this crate yet: `SelectionCommand`, `Transcribe`,
//! `ListModels`, `SetActiveModel`, `Benchmark`.

pub const SERVICE_NAME: &str = "dev.narad.Daemon";
pub const OBJECT_PATH: &str = "/dev/narad/Daemon";
/// D-Bus property name for floating HUD visibility (ARCHITECTURE §6).
pub const PROP_HUD_VISIBLE: &str = "HudVisible";

pub mod daemon_iface;
pub mod types;

/// Client proxy for Phase 1 methods, signals, and properties.
///
/// The `#[zbus::proxy]` definition in [`daemon_iface`] uses an internal trait name `Daemon`;
/// zbus expands it into this type (the trait itself is not exported).
pub use daemon_iface::DaemonProxy;
pub use types::{HistoryEntry, SessionState};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_name_stable() {
        assert_eq!(SERVICE_NAME, "dev.narad.Daemon");
        assert_eq!(OBJECT_PATH, "/dev/narad/Daemon");
    }

    #[test]
    fn session_state_values_match_architecture() {
        assert_eq!(SessionState::IDLE, "idle");
        assert_eq!(SessionState::LISTENING, "listening");
        assert_eq!(SessionState::PROCESSING, "processing");
        assert_eq!(SessionState::ERROR, "error");
        assert_eq!(SessionState::COPIED, "copied");
    }

    #[test]
    fn session_state_brief_outcomes_fr_dot_2() {
        assert_eq!(SessionState::INSERTED, "inserted");
        assert_eq!(SessionState::CANCELLED, "cancelled");
    }

    #[test]
    fn session_state_undo_outcome_fr_ins_5() {
        assert_eq!(SessionState::UNDONE, "undone");
        assert_ne!(SessionState::UNDONE, SessionState::INSERTED);
        assert_ne!(SessionState::UNDONE, SessionState::COPIED);
        assert_ne!(SessionState::UNDONE, SessionState::CANCELLED);
        assert_ne!(SessionState::UNDONE, SessionState::ERROR);
    }

    #[test]
    fn hud_visible_property_name_matches_architecture() {
        assert_eq!(PROP_HUD_VISIBLE, "HudVisible");
    }

    #[test]
    fn report_focus_is_on_the_daemon_proxy() {
        async fn assert_sig(proxy: &DaemonProxy<'_>) {
            let _ = proxy.report_focus("").await;
        }
        let _ = assert_sig;
    }

    #[test]
    fn paste_last_is_on_the_daemon_proxy() {
        async fn assert_sig(proxy: &DaemonProxy<'_>) {
            let _ = proxy.paste_last().await;
        }
        let _ = assert_sig;
    }

    #[test]
    fn undo_last_is_on_the_daemon_proxy() {
        async fn assert_sig(proxy: &DaemonProxy<'_>) {
            let _ = proxy.undo_last().await;
            let _ = proxy.outcome_detail().await;
        }
        let _ = assert_sig;
    }

    #[test]
    fn paused_is_writable_on_the_daemon_proxy() {
        async fn assert_sig(proxy: &DaemonProxy<'_>) {
            let _ = proxy.set_paused(true).await;
            let _ = proxy.paused().await;
        }
        let _ = assert_sig;
    }
}
