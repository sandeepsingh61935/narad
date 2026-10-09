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
) -> Result<Vec<HistoryEntry>, String> {
    daemon_proxy()
        .await?
        .history(&query, offset, limit)
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
            delete_entries,
            load_config,
            save_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
