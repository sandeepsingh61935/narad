//! Client proxy and method signatures for `dev.narad.Daemon` (Phase 1).
//!
//! Phase 2+ D-Bus methods still omitted: `SelectionCommand`, `Transcribe`,
//! `ListModels`, `SetActiveModel`, `Benchmark`.
//! `PasteLast` is included (FR-INS-4).
//! `UndoLast` is included (FR-INS-5).
//! `ReportFocus` is included (FR-CTX-1).

use crate::types::HistoryEntry;
use zbus::proxy;

/// Phase 1 D-Bus interface for the Narad daemon (ARCHITECTURE §6).
///
/// Use [`DaemonProxy`] on clients. On the daemon, implement the same method names with
/// [`zbus::interface`](zbus::interface) (see `naradd` `ipc` module).
#[proxy(
    interface = "dev.narad.Daemon",
    default_service = "dev.narad.Daemon",
    default_path = "/dev/narad/Daemon"
)]
pub trait Daemon {
    async fn start(&self, mode: &str) -> zbus::Result<()>;

    async fn stop(&self) -> zbus::Result<()>;

    async fn cancel(&self) -> zbus::Result<()>;

    async fn toggle(&self, mode: &str) -> zbus::Result<()>;

    /// List or search history. Empty filter strings are omitted (FR-HIS-3).
    ///
    /// `from_day` and `to_day` are `YYYY-MM-DD` in the daemon's local timezone,
    /// inclusive. A bad date fails and returns no rows.
    async fn history(
        &self,
        query: &str,
        offset: u32,
        limit: u32,
        from_day: &str,
        to_day: &str,
        mode: &str,
        app_id: &str,
        src_lang: &str,
    ) -> zbus::Result<Vec<HistoryEntry>>;

    async fn reinsert_entry(&self, id: i64) -> zbus::Result<()>;

    /// Re-insert the newest history row's final text (FR-INS-4). No arguments.
    async fn paste_last(&self) -> zbus::Result<()>;

    /// Remove the last insertion when focus is unchanged (FR-INS-5). No arguments.
    async fn undo_last(&self) -> zbus::Result<()>;

    /// Deleted text after undo, or the reason undo was refused. Empty otherwise.
    #[zbus(property)]
    fn outcome_detail(&self) -> zbus::Result<String>;

    /// Clients may pass a slice; the daemon interface deserializes as `Vec<i64>`.
    async fn delete_entries(&self, ids: &[i64]) -> zbus::Result<()>;

    /// Focused application id from the shell extension (FR-CTX-1).
    ///
    /// Empty string clears the cached focus. The daemon snapshots this when
    /// recording starts (history `app_id`) and again when text is pasted.
    async fn report_focus(&self, app_id: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    fn state_changed(&self, state: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    fn level(&self, rms: f64) -> zbus::Result<()>;

    #[zbus(signal)]
    fn entry_added(&self, id: i64) -> zbus::Result<()>;

    #[zbus(property)]
    fn mode(&self) -> zbus::Result<String>;

    /// When true, a finished dictation still pastes but writes no history row
    /// and emits no `EntryAdded` (FR-HIS-7). Explicit transcribe and re-insert
    /// are not gated. Writable; proxy exposes `set_paused`.
    #[zbus(property)]
    fn paused(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn set_paused(&self, value: bool) -> zbus::Result<()>;

    /// Floating HUD visibility (ARCHITECTURE §6 / FR-DOT-3). Writable; proxy exposes `set_hud_visible`.
    #[zbus(property)]
    fn hud_visible(&self) -> zbus::Result<bool>;
}
