//! Narad dashboard — thin Tauri command layer over `narad-dbus` + local config.

mod config;

use config::DashboardConfig;
use narad_dbus::{DaemonProxy, HistoryEntry};
use zbus::Connection;

async fn daemon_proxy() -> Result<DaemonProxy<'static>, String> {
    let connection = Connection::session()
        .await
        .map_err(|e| format!("session D-Bus: {e}"))?;
    DaemonProxy::new(&connection)
        .await
        .map_err(|e| format!("dev.narad.Daemon: {e}"))
}

/// List history rows from the session-bus daemon (`History` D-Bus method).
#[tauri::command]
async fn list_history(
    query: String,
    offset: u32,
    limit: u32,
    from_day: Option<String>,
    to_day: Option<String>,
    mode: Option<String>,
    app_id: Option<String>,
    src_lang: Option<String>,
) -> Result<Vec<HistoryEntry>, String> {
    let from_day = from_day.unwrap_or_default();
    let to_day = to_day.unwrap_or_default();
    let mode = mode.unwrap_or_default();
    let app_id = app_id.unwrap_or_default();
    let src_lang = src_lang.unwrap_or_default();
    daemon_proxy()
        .await?
        .history(
            &query,
            offset,
            limit,
            &from_day,
            &to_day,
            &mode,
            &app_id,
            &src_lang,
        )
        .await
        .map_err(|e| format!("History: {e}"))
}

/// Re-insert a history entry into the focused app (`ReinsertEntry`).
#[tauri::command]
async fn reinsert_entry(id: i64) -> Result<(), String> {
    daemon_proxy()
        .await?
        .reinsert_entry(id)
        .await
        .map_err(|e| format!("ReinsertEntry: {e}"))
}

/// Retained WAV bytes (`ReadAudio`). The dashboard plays them; the daemon does not.
#[tauri::command]
async fn read_audio(id: i64) -> Result<Vec<u8>, String> {
    daemon_proxy()
        .await?
        .read_audio(id)
        .await
        .map_err(|e| format!("ReadAudio: {e}"))
}

/// New history row from retained audio (`Retranscribe`). Does not paste.
#[tauri::command]
async fn retranscribe(id: i64, mode: String, model_id: String) -> Result<i64, String> {
    daemon_proxy()
        .await?
        .retranscribe(id, &mode, &model_id)
        .await
        .map_err(|e| format!("Retranscribe: {e}"))
}

/// Set one row's pin (`SetPinned`). Does not filter the list.
#[tauri::command]
async fn set_pinned(id: i64, pinned: bool) -> Result<(), String> {
    daemon_proxy()
        .await?
        .set_pinned(id, pinned)
        .await
        .map_err(|e| format!("SetPinned: {e}"))
}

/// Replace one entry's tags (`SetTags`).
#[tauri::command]
async fn set_tags(id: i64, names: Vec<String>) -> Result<(), String> {
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    daemon_proxy()
        .await?
        .set_tags(id, &refs)
        .await
        .map_err(|e| format!("SetTags: {e}"))
}

/// Delete history entries by id (`DeleteEntries`).
#[tauri::command]
async fn delete_entries(ids: Vec<i64>) -> Result<(), String> {
    daemon_proxy()
        .await?
        .delete_entries(&ids)
        .await
        .map_err(|e| format!("DeleteEntries: {e}"))
}

/// Load `~/.config/narad/config.toml` (defaults if missing). FR-HIS-9.
#[tauri::command]
fn load_config() -> Result<DashboardConfig, String> {
    config::load_config()
}

/// `Paused` and `Mode` as already exposed on `dev.narad.Daemon`. No new method.
#[derive(serde::Serialize)]
struct SessionStatus {
    paused: bool,
    mode: String,
}

#[tauri::command]
async fn session_status() -> Result<SessionStatus, String> {
    let proxy = daemon_proxy().await?;
    let paused = proxy
        .paused()
        .await
        .map_err(|e| format!("Paused: {e}"))?;
    let mode = proxy.mode().await.map_err(|e| format!("Mode: {e}"))?;
    Ok(SessionStatus { paused, mode })
}

/// Write settings to `~/.config/narad/config.toml`. FR-HIS-9.
#[tauri::command]
fn save_config(config: DashboardConfig) -> Result<(), String> {
    config::save_config(&config)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_history,
            reinsert_entry,
            read_audio,
            retranscribe,
            set_pinned,
            set_tags,
            delete_entries,
            load_config,
            save_config,
            session_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
