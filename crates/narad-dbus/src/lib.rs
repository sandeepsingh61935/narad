//! Shared D-Bus contract for `dev.narad.Daemon`.
//!
//! Interface sketch: `docs/ARCHITECTURE.md` §6.
//!
//! ## Phase 1 surface
//!
//! Methods: `Start`, `Stop`, `Cancel`, `Toggle`, `SelectionCommand` (FR-INS-6),
//! `History`, `ReinsertEntry`, `DeleteEntries`,
//! `PasteLast` (FR-INS-4), `UndoLast` (FR-INS-5), `ReportFocus` (FR-CTX-1),
//! `ReadAudio` and `Retranscribe` (FR-HIS-5), `SetPinned`, `SetTags`,
//! `Export`, and `DeleteAll` (FR-HIS-6), `Stats` (FR-HIS-8),
//! `Transcribe` (FR-ASR-6).
//! Signals: `StateChanged`, `Level`, `EntryAdded`.
//! Properties: `Mode` (writable), `Paused` (writable), `HudVisible` (writable).
//!
//! ## Deferred (Phase 2+)
//!
//! Not defined in this crate yet:
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
pub use types::{HistoryEntry, HistoryStats, SessionState};

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
    fn selection_command_is_on_the_daemon_proxy() {
        async fn assert_sig(proxy: &DaemonProxy<'_>) {
            let _ = proxy.selection_command().await;
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

    #[test]
    fn mode_is_writable_on_the_daemon_proxy() {
        async fn assert_sig(proxy: &DaemonProxy<'_>) {
            let _ = proxy.set_mode("code").await;
            let _ = proxy.mode().await;
        }
        let _ = assert_sig;
    }

    #[test]
    fn read_audio_and_retranscribe_are_on_the_daemon_proxy() {
        async fn assert_sig(proxy: &DaemonProxy<'_>) {
            let _ = proxy.read_audio(1).await;
            let _ = proxy
                .retranscribe(1, "email", "parakeet-ctc-110m-int8")
                .await;
        }
        let _ = assert_sig;
    }

    #[test]
    fn set_pinned_and_set_tags_are_on_the_daemon_proxy() {
        async fn assert_sig(proxy: &DaemonProxy<'_>) {
            let _ = proxy.set_pinned(1, true).await;
            let _ = proxy.set_tags(1, &[" a ", "", "a", "A"]).await;
        }
        let _ = assert_sig;
    }

    #[test]
    fn export_and_delete_all_are_on_the_daemon_proxy() {
        async fn assert_sig(proxy: &DaemonProxy<'_>) {
            let _ = proxy.export(&[7], "json").await;
            let _ = proxy.delete_all().await;
        }
        let _ = assert_sig;
    }

    #[test]
    fn stats_is_on_the_daemon_proxy() {
        async fn assert_sig(proxy: &DaemonProxy<'_>) {
            let stats = proxy.stats("day").await.expect("stats");
            let _ = (
                stats.words,
                stats.utterances,
                stats.speaking_wpm,
                stats.time_saved_ms,
                stats.from_day,
                stats.to_day,
            );
        }
        let _ = assert_sig;
    }

    #[test]
    fn transcribe_is_on_the_daemon_proxy() {
        async fn assert_sig(proxy: &DaemonProxy<'_>) {
            let id: i64 = proxy
                .transcribe("/tmp/clip.wav", "plain", "")
                .await
                .expect("id");
            let _ = id;
        }
        let _ = assert_sig;
    }

    #[test]
    fn history_entry_tags_follow_kind() {
        let entry = HistoryEntry {
            id: 4,
            created_at: 5,
            duration_ms: 16,
            mode: "email".into(),
            src_lang: "en".into(),
            dst_lang: "en".into(),
            asr_model: "parakeet-ctc-110m-int8".into(),
            mt_model: None,
            llm_model: None,
            app_id: Some("org.gnome.TextEditor".into()),
            raw_text: "raw".into(),
            final_text: "final".into(),
            insert_status: "failed".into(),
            cleanup_flagged: false,
            audio_path: None,
            pinned: false,
            kind: "dictation".into(),
            tags: vec!["a".into(), "A".into()],
        };
        assert_eq!(entry.tags, ["a", "A"]);
    }

    #[test]
    fn history_entry_kind_is_retranscribe_when_set() {
        let entry = HistoryEntry {
            id: 4,
            created_at: 5,
            duration_ms: 16,
            mode: "email".into(),
            src_lang: "en".into(),
            dst_lang: "en".into(),
            asr_model: "parakeet-ctc-110m-int8".into(),
            mt_model: None,
            llm_model: None,
            app_id: Some("org.gnome.TextEditor".into()),
            raw_text: "raw".into(),
            final_text: "final".into(),
            insert_status: "failed".into(),
            cleanup_flagged: false,
            audio_path: None,
            pinned: false,
            kind: "retranscribe".into(),
            tags: Vec::new(),
        };
        assert_eq!(entry.kind, "retranscribe");
    }
}
