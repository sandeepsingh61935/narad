//! Phase 0 throwaway spike: GlobalShortcuts portal press + release.
//!
//! Proves `org.freedesktop.portal.GlobalShortcuts` emits `Activated` and
//! `Deactivated` for a bound shortcut (target: Ctrl+Alt+Space).
//!
//! xdg-desktop-portal ≥ 1.20 requires host (non-Flatpak) apps to register an
//! app id via `org.freedesktop.host.portal.Registry` before CreateSession.
//! GNOME resolves that id through a `.desktop` file whose `Exec=` is a real
//! path (not a bare name missing from `$PATH`).

use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Context;
use ashpd::desktop::global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, NewShortcut};
use ashpd::desktop::CreateSessionOptions;
use ashpd::AppID;
use futures_util::StreamExt;

/// Reverse-DNS id; must match `{APP_ID}.desktop` (sans suffix).
const APP_ID: &str = "dev.narad.SpikeHotkeys";
const SHORTCUT_ID: &str = "narad-push-to-talk";
/// Freedesktop shortcuts spec: modifiers + xkbcommon keysym name (no `XKB_KEY_` prefix).
const PREFERRED_TRIGGER: &str = "CTRL+ALT+space";

fn applications_dir() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join("applications"));
        }
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share/applications"))
}

/// Write `~/.local/share/applications/{APP_ID}.desktop` with absolute Exec=.
fn ensure_desktop_file(exe: &std::path::Path) -> anyhow::Result<PathBuf> {
    let apps = applications_dir().context("cannot resolve ~/.local/share/applications")?;
    std::fs::create_dir_all(&apps).context("mkdir applications")?;
    let dest = apps.join(format!("{APP_ID}.desktop"));
    let exe_str = exe
        .to_str()
        .context("executable path is not valid UTF-8")?;
    // Escape only what the desktop-entry spec requires for Exec.
    let exec = exe_str.replace('\\', "\\\\").replace('"', "\\\"");
    let body = format!(
        "\
[Desktop Entry]
Version=1.0
Type=Application
Name=Narad Hotkeys Spike
Comment=Phase 0 GlobalShortcuts portal probe (throwaway)
Exec={exec}
TryExec={exec}
Icon=input-keyboard
Terminal=true
Categories=Development;
StartupNotify=false
"
    );
    std::fs::write(&dest, body).with_context(|| format!("write {}", dest.display()))?;

    // Help GIO / portal see the new file without a logout (best-effort).
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&apps)
        .status();

    Ok(dest)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let exe = std::env::current_exe().context("current_exe")?;
    let desktop = ensure_desktop_file(&exe)?;
    tracing::info!(
        path = %desktop.display(),
        exe = %exe.display(),
        "installed host app .desktop for portal"
    );

    let app_id = AppID::from_str(APP_ID).context("invalid APP_ID")?;
    ashpd::register_host_app(app_id)
        .await
        .context(
            "Registry.Register failed — need xdg-desktop-portal ≥ 1.20 and a \
             .desktop whose Exec= points at this binary (absolute path)",
        )?;
    tracing::info!(app_id = APP_ID, "registered host app with portal Registry");

    let portal = GlobalShortcuts::new()
        .await
        .context("connect to org.freedesktop.portal.GlobalShortcuts (is xdg-desktop-portal running?)")?;

    tracing::info!(version = portal.version(), "connected to GlobalShortcuts portal");

    let session = portal
        .create_session(CreateSessionOptions::default())
        .await
        .context("CreateSession failed after Registry.Register")?;

    tracing::info!("CreateSession OK");

    let shortcut = NewShortcut::new(SHORTCUT_ID, "Narad spike: probe push-to-talk hotkey")
        .preferred_trigger(Some(PREFERRED_TRIGGER));

    tracing::info!(
        "binding shortcut (portal UI may appear — approve or adjust the trigger)"
    );

    let request = portal
        .bind_shortcuts(&session, &[shortcut], None, BindShortcutsOptions::default())
        .await
        .context("BindShortcuts request failed")?;

    let bound = request
        .response()
        .context("BindShortcuts portal response failed (did you dismiss the dialog?)")?;

    for s in bound.shortcuts() {
        println!(
            "bound shortcut id={} trigger={}",
            s.id(),
            s.trigger_description()
        );
    }

    if bound.shortcuts().is_empty() {
        anyhow::bail!(
            "portal bound zero shortcuts — try again and accept the bind dialog, or pick Ctrl+Alt+Space in the UI"
        );
    }

    let mut activated = portal.receive_activated().await?;
    let mut deactivated = portal.receive_deactivated().await?;

    println!(
        "Listening for {PREFERRED_TRIGGER} — hold and release; expect Activated then Deactivated. Ctrl+C to exit."
    );

    loop {
        tokio::select! {
            event = activated.next() => {
                match event {
                    Some(ev) => println!("Activated: shortcut_id={}", ev.shortcut_id()),
                    None => anyhow::bail!("Activated signal stream ended"),
                }
            }
            event = deactivated.next() => {
                match event {
                    Some(ev) => println!("Deactivated: shortcut_id={}", ev.shortcut_id()),
                    None => anyhow::bail!("Deactivated signal stream ended"),
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("Exiting.");
                break;
            }
        }
    }

    Ok(())
}
