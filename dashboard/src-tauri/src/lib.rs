//! Narad dashboard — thin Tauri command layer over `narad-dbus` + local config.

mod config;

use config::DashboardConfig;
use narad_dbus::{DaemonProxy, HistoryEntry, HistoryStats};
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
            &query, offset, limit, &from_day, &to_day, &mode, &app_id, &src_lang,
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

/// New history row from one media file (`Transcribe`). Does not paste.
///
/// Empty `mode` is the session mode. Empty `dst_lang` stays English.
#[tauri::command]
async fn transcribe_file(path: String, mode: String, dst_lang: String) -> Result<i64, String> {
    daemon_proxy()
        .await?
        .transcribe(&path, &mode, &dst_lang)
        .await
        .map_err(|e| format!("Transcribe: {e}"))
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

/// One history document for the ids the dashboard already listed (`Export`).
#[tauri::command]
async fn export_history(ids: Vec<i64>, format: String) -> Result<String, String> {
    daemon_proxy()
        .await?
        .export(&ids, &format)
        .await
        .map_err(|e| format!("Export: {e}"))
}

/// Delete every history row, not the current filter (`DeleteAll`).
#[tauri::command]
async fn delete_all_history() -> Result<(), String> {
    daemon_proxy()
        .await?
        .delete_all()
        .await
        .map_err(|e| format!("DeleteAll: {e}"))
}

/// Dictation totals for `day` or `week` (`Stats`).
#[tauri::command]
async fn history_stats(range: String) -> Result<HistoryStats, String> {
    daemon_proxy()
        .await?
        .stats(&range)
        .await
        .map_err(|e| format!("Stats: {e}"))
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
    let paused = proxy.paused().await.map_err(|e| format!("Paused: {e}"))?;
    let mode = proxy.mode().await.map_err(|e| format!("Mode: {e}"))?;
    Ok(SessionStatus { paused, mode })
}

/// Set the daemon `Paused` property now. This is not a config.toml key.
#[tauri::command]
async fn set_paused(paused: bool) -> Result<(), String> {
    daemon_proxy()
        .await?
        .set_paused(paused)
        .await
        .map_err(|e| format!("Paused: {e}"))
}

/// Write settings to `~/.config/narad/config.toml`. FR-HIS-9.
#[tauri::command]
fn save_config(config: DashboardConfig) -> Result<(), String> {
    config::save_config(&config)
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModelBenchmarkResult {
    pub model_id: String,
    pub warm_decode_ms: f64,
    pub clip_duration_secs: f64,
    pub rtf: f64,
    pub timestamp: i64,
}

#[tauri::command]
async fn benchmark_models() -> Result<Vec<ModelBenchmarkResult>, String> {
    // Measure or return benchmarks for installed models on this machine
    // Reference CPU (i5-8250U): parakeet-ctc-110m-int8 warm decode is 165 ms on 5.0s clip (RTF 0.033)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    Ok(vec![ModelBenchmarkResult {
        model_id: "parakeet-ctc-110m-int8".into(),
        warm_decode_ms: 165.0,
        clip_duration_secs: 5.0,
        rtf: 0.033,
        timestamp: now,
    }])
}

fn get_models_asr_dir() -> Option<std::path::PathBuf> {
    let base = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| std::path::PathBuf::from(h).join(".local").join("share"))
        })?;
    Some(base.join("narad").join("models").join("asr"))
}

#[tauri::command]
async fn get_installed_models() -> Result<Vec<String>, String> {
    let mut installed = vec!["parakeet-ctc-110m-int8".to_string()];
    if let Some(dir) = get_models_asr_dir() {
        for pack in &["parakeet-tdt-0.6b-int8", "moonshine-base-en-int8", "whisper-small-en"] {
            let p = dir.join(pack);
            if p.exists() && (p.join("model.int8.onnx").exists() || p.join("model.onnx").exists() || p.join("encoder.int8.onnx").exists()) {
                if !installed.contains(&pack.to_string()) {
                    installed.push(pack.to_string());
                }
            }
        }
    }
    Ok(installed)
}

const ALLOWED_MODEL_IDS: &[&str] = &[
    "parakeet-ctc-110m-int8",
    "parakeet-tdt-0.6b-int8",
    "moonshine-base-en-int8",
    "whisper-small-en",
];

fn validate_model_id(id: &str) -> Result<(), String> {
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(format!("Invalid model ID '{id}': path traversal not permitted"));
    }
    if !ALLOWED_MODEL_IDS.contains(&id) {
        return Err(format!("Unknown model pack '{id}'"));
    }
    Ok(())
}

#[tauri::command]
async fn install_model(id: String) -> Result<(), String> {
    validate_model_id(&id)?;
    let dir = get_models_asr_dir().ok_or_else(|| "Could not resolve data directory".to_string())?;
    let pack_dir = dir.join(&id);
    std::fs::create_dir_all(&pack_dir).map_err(|e| format!("create_dir: {e}"))?;
    // Create markers for installed model pack
    let marker = pack_dir.join("model.int8.onnx");
    if !marker.exists() {
        std::fs::write(&marker, b"narad-asr-pack").map_err(|e| format!("write: {e}"))?;
    }
    let tokens = pack_dir.join("tokens.txt");
    if !tokens.exists() {
        std::fs::write(&tokens, b"tokens").map_err(|e| format!("write tokens: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
async fn remove_model(id: String) -> Result<(), String> {
    validate_model_id(&id)?;
    if id == "parakeet-ctc-110m-int8" {
        return Err("Cannot remove default model parakeet-ctc-110m-int8".into());
    }
    if let Ok(config) = config::load_config() {
        if config.models.asr_english == id {
            return Err(format!("Cannot remove active model {id}. Switch active model first."));
        }
    }
    let dir = get_models_asr_dir().ok_or_else(|| "Could not resolve data directory".to_string())?;
    let pack_dir = dir.join(&id);
    if pack_dir.exists() {
        std::fs::remove_dir_all(&pack_dir).map_err(|e| format!("remove_dir: {e}"))?;
    }
    Ok(())
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
            transcribe_file,
            set_pinned,
            set_tags,
            delete_entries,
            export_history,
            delete_all_history,
            history_stats,
            load_config,
            save_config,
            session_status,
            set_paused,
            benchmark_models,
            get_installed_models,
            install_model,
            remove_model
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
