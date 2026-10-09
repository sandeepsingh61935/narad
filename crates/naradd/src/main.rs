//! Narad daemon entrypoint: config, warm ASR, D-Bus, hotkeys, dictation pipeline.

mod asr;
mod audio;
mod config;
mod hotkeys;
mod insert;
mod ipc;
mod paths;
mod pipeline;

use anyhow::{Context, Result};
use config::{load_or_create, resolve_config_path, NaradConfig};
use ipc::DaemonIpc;
use narad_core::store::HistoryStore;
use narad_dbus::{OBJECT_PATH, SERVICE_NAME};
use paths::{ensure_data_dir, resolve_db_path};
use pipeline::PipelineHandle;
use tracing::{info, warn, Level};
use tracing_subscriber::EnvFilter;
use zbus::connection;
use zbus::fdo::DBusProxy;

/// Exit quietly when another naradd already owns the session bus name.
/// GNOME may launch `{APP_ID}.desktop` (portal Registry / GIO); that must not
/// steal the systemd user service instance.
async fn exit_if_already_running() -> Result<bool> {
    let conn = connection::Builder::session()
        .context("connect to session D-Bus for single-instance check")?
        .build()
        .await
        .context("build session connection for single-instance check")?;
    let dbus = DBusProxy::new(&conn)
        .await
        .context("DBusProxy for single-instance check")?;
    let owned = dbus
        .name_has_owner(SERVICE_NAME.try_into().context("SERVICE_NAME")?)
        .await
        .context("NameHasOwner")?;
    if owned {
        info!(
            service = SERVICE_NAME,
            "another naradd already owns the bus name; exiting (single-instance)"
        );
        return Ok(true);
    }
    Ok(false)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    if exit_if_already_running().await? {
        return Ok(());
    }

    let cfg = match resolve_config_path() {
        Some(path) => load_or_create(&path)?,
        None => {
            warn!("XDG config path unavailable; using in-memory defaults");
            NaradConfig::default()
        }
    };

    info!(
        hotkey = %cfg.activation.dictate_hotkey,
        hold_tap_threshold_ms = cfg.activation.hold_tap_threshold_ms,
        active_mode = %cfg.general.active_mode,
        sounds_enabled = cfg.audio.sounds_enabled,
        asr = %cfg.models.asr_english,
        "naradd config loaded"
    );

    let data_dir = ensure_data_dir().context("create Narad data directory")?;
    let db_path = resolve_db_path()
        .filter(|p| p.parent().is_some())
        .unwrap_or_else(|| data_dir.join("narad.db"));
    let store = HistoryStore::open(&db_path)
        .with_context(|| format!("open history database {}", db_path.display()))?;
    info!(db = %db_path.display(), "history store ready");

    asr::ensure_default_pack_installed(&cfg.models.asr_english);
    let asr_engine = asr::try_load(&cfg.models.asr_english);
    let pipeline = PipelineHandle::new(cfg.clone(), store, asr_engine);
    let daemon = DaemonIpc::new(pipeline.clone());

    // Do not allow portal/GIO-launched copies to replace the systemd instance.
    let conn = connection::Builder::session()
        .context("connect to session D-Bus")?
        .allow_name_replacements(false)
        .replace_existing_names(false)
        .name(SERVICE_NAME)
        .context("claim D-Bus name")?
        .serve_at(OBJECT_PATH, daemon)
        .context("export daemon object")?
        .build()
        .await
        .context("start D-Bus connection")?;

    pipeline.set_connection(conn.clone());

    info!(
        service = SERVICE_NAME,
        path = OBJECT_PATH,
        "naradd listening on session bus"
    );

    let hotkeys_pipeline = pipeline.clone();
    tokio::spawn(async move {
        if let Err(err) = hotkeys::run(hotkeys_pipeline).await {
            warn!(error = %err, "hotkeys task ended");
        }
    });

    tokio::signal::ctrl_c().await?;
    info!("shutting down");
    drop(conn);
    Ok(())
}
