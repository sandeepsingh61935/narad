//! GlobalShortcuts portal glue (FR-ACT-1). Host Registry + `.desktop` required on portal ≥ 1.20.
//!
//! GNOME may emit repeated `Activated` signals while the key is held (keyboard repeat).
//! Those must not toggle Start/Stop — only the first press and the matching release count.

use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;

use anyhow::{Context, Result};
use ashpd::desktop::global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, NewShortcut};
use ashpd::desktop::CreateSessionOptions;
use ashpd::AppID;
use futures_util::StreamExt;
use tracing::{info, warn};

use crate::pipeline::PipelineHandle;

/// Reverse-DNS id; must match `{APP_ID}.desktop`.
pub const APP_ID: &str = "dev.narad.Narad";
const DICTATE_ID: &str = "narad-dictate";
const PASTE_LAST_ID: &str = "narad-paste-last";
const UNDO_LAST_ID: &str = "narad-undo-last";

/// Paste-last is a press. Key-repeat while held must not paste again. Release is not hold/tap.
pub(crate) fn paste_last_should_fire(already_down: bool) -> bool {
    !already_down
}

fn applications_dir() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join("applications"));
        }
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share/applications"))
}

/// Write `~/.local/share/applications/dev.narad.Narad.desktop` with absolute Exec=.
pub fn ensure_desktop_file(exe: &std::path::Path) -> Result<PathBuf> {
    let apps = applications_dir().context("cannot resolve applications dir")?;
    std::fs::create_dir_all(&apps).context("mkdir applications")?;
    let dest = apps.join(format!("{APP_ID}.desktop"));
    let exe_str = exe
        .to_str()
        .context("executable path is not valid UTF-8")?;
    let exec = exe_str.replace('\\', "\\\\").replace('"', "\\\"");
    // NoDisplay: portal Registry needs a resolvable .desktop; hide from app grids.
    // naradd itself is single-instance (exits if bus name already owned).
    let body = format!(
        "\
[Desktop Entry]
Version=1.0
Type=Application
Name=Narad
Comment=Local voice-to-text dictation daemon
Exec={exec}
TryExec={exec}
Icon=audio-input-microphone
Terminal=false
NoDisplay=true
Categories=Utility;AudioVideo;
StartupNotify=false
"
    );
    std::fs::write(&dest, body).with_context(|| format!("write {}", dest.display()))?;
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&apps)
        .status();
    Ok(dest)
}

/// Convert config hotkey (`Ctrl+Alt+Space`) to portal preferred_trigger (`CTRL+ALT+space`).
pub fn to_portal_trigger(hotkey: &str) -> String {
    let mut parts: Vec<String> = hotkey
        .split('+')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(|p| {
            let upper = p.to_ascii_uppercase();
            match upper.as_str() {
                "CTRL" | "CONTROL" => "CTRL".into(),
                "ALT" => "ALT".into(),
                "SHIFT" => "SHIFT".into(),
                "SUPER" | "META" | "WIN" => "SUPER".into(),
                "SPACE" => "space".into(),
                other => {
                    if other.len() == 1 {
                        other.to_ascii_lowercase()
                    } else {
                        p.to_ascii_lowercase()
                    }
                }
            }
        })
        .collect();
    if parts.is_empty() {
        parts.push("CTRL".into());
        parts.push("ALT".into());
        parts.push("space".into());
    }
    parts.join("+")
}

/// Bind dictate (hold/tap), paste-last, and undo-last, and forward portal events into the pipeline.
pub async fn run(pipeline: PipelineHandle) -> Result<()> {
    let exe = std::env::current_exe().context("current_exe")?;
    let desktop = ensure_desktop_file(&exe)?;
    info!(path = %desktop.display(), exe = %exe.display(), "installed host app .desktop");

    let app_id = AppID::from_str(APP_ID).context("invalid APP_ID")?;
    ashpd::register_host_app(app_id)
        .await
        .context("Registry.Register failed")?;
    info!(app_id = APP_ID, "registered host app with portal Registry");

    let (trigger, paste_trigger, undo_trigger, threshold_ms) = {
        let cfg = pipeline.config();
        (
            to_portal_trigger(&cfg.activation.dictate_hotkey),
            to_portal_trigger(&cfg.activation.paste_last_hotkey),
            to_portal_trigger(&cfg.activation.undo_last_hotkey),
            cfg.activation.hold_tap_threshold_ms,
        )
    };

    let portal = GlobalShortcuts::new()
        .await
        .context("connect GlobalShortcuts portal")?;
    info!(
        version = portal.version(),
        trigger = %trigger,
        paste_last = %paste_trigger,
        undo_last = %undo_trigger,
        "GlobalShortcuts connected"
    );

    let session = portal
        .create_session(CreateSessionOptions::default())
        .await
        .context("GlobalShortcuts.CreateSession")?;

    let dictate = NewShortcut::new(DICTATE_ID, "Narad dictate (hold or tap)")
        .preferred_trigger(Some(trigger.as_str()));
    let paste_last = NewShortcut::new(PASTE_LAST_ID, "Narad paste last transcription")
        .preferred_trigger(Some(paste_trigger.as_str()));
    let undo_last = NewShortcut::new(UNDO_LAST_ID, "Narad undo last insertion")
        .preferred_trigger(Some(undo_trigger.as_str()));

    let request = portal
        .bind_shortcuts(
            &session,
            &[dictate, paste_last, undo_last],
            None,
            BindShortcutsOptions::default(),
        )
        .await
        .context("BindShortcuts")?;
    let bound = request.response().context("BindShortcuts response")?;
    for s in bound.shortcuts() {
        info!(
            id = s.id(),
            trigger = s.trigger_description(),
            "bound shortcut"
        );
    }
    if bound.shortcuts().is_empty() {
        warn!("portal bound zero shortcuts — approve the bind dialog or set the hotkey in Settings");
    }

    let mut activated = portal.receive_activated().await?;
    let mut deactivated = portal.receive_deactivated().await?;
    // True between first Activated and matching Deactivated (suppresses key-repeat Activates).
    let mut key_down = false;
    let mut paste_down = false;
    let mut undo_down = false;
    let mut press_at: Option<Instant> = None;
    // After a second-tap stop, ignore the following Deactivated so it does not re-interpret hold.
    let mut suppress_deactivate = false;

    info!(
        trigger = %trigger,
        threshold_ms,
        paste_last = %paste_trigger,
        undo_last = %undo_trigger,
        "hotkeys listening (dictate hold/tap; paste-last and undo-last on press; key-repeat ignored)"
    );

    loop {
        tokio::select! {
            event = activated.next() => {
                match event {
                    Some(ev) => {
                        if ev.shortcut_id() == PASTE_LAST_ID {
                            let fire = paste_last_should_fire(paste_down);
                            paste_down = true;
                            if fire {
                                info!("paste-last hotkey → PasteLast");
                                pipeline.request_paste_last();
                            }
                            continue;
                        }
                        if ev.shortcut_id() == UNDO_LAST_ID {
                            let fire = paste_last_should_fire(undo_down);
                            undo_down = true;
                            if fire {
                                info!("undo-last hotkey → UndoLast");
                                pipeline.request_undo_last();
                            }
                            continue;
                        }
                        if ev.shortcut_id() != DICTATE_ID {
                            continue;
                        }
                        // Keyboard repeat while held: portal re-fires Activated. Ignore.
                        if key_down {
                            continue;
                        }
                        key_down = true;
                        press_at = Some(Instant::now());

                        if pipeline.is_listening() {
                            // Second tap (key was up, listening from prior tap) → stop.
                            info!("hotkey second tap → Stop");
                            suppress_deactivate = true;
                            pipeline.request_stop();
                        } else {
                            info!("hotkey press → Start");
                            let t0 = Instant::now();
                            let pipe = pipeline.clone();
                            pipeline.request_hotkey_press();
                            // NFR-P-1: Activated → listening (mic open + state). Poll; do not block portal loop.
                            tokio::spawn(async move {
                                for _ in 0..500 {
                                    if pipe.is_listening() {
                                        let ms = t0.elapsed().as_secs_f64() * 1000.0;
                                        info!(nfr_p1_ms = format!("{ms:.3}"), "NFR-P-1 hotkey→listening");
                                        return;
                                    }
                                    tokio::time::sleep(std::time::Duration::from_micros(200)).await;
                                }
                                warn!("NFR-P-1: listening not observed within 100 ms poll window");
                            });
                        }
                    }
                    None => {
                        warn!("Activated stream ended");
                        break;
                    }
                }
            }
            event = deactivated.next() => {
                match event {
                    Some(ev) => {
                        if ev.shortcut_id() == PASTE_LAST_ID {
                            paste_down = false;
                            continue;
                        }
                        if ev.shortcut_id() == UNDO_LAST_ID {
                            undo_down = false;
                            continue;
                        }
                        if ev.shortcut_id() != DICTATE_ID {
                            continue;
                        }
                        if !key_down {
                            // Spurious deactivate (or repeat noise after we already cleared).
                            continue;
                        }
                        let held_ms = press_at
                            .take()
                            .map(|t| t.elapsed().as_millis() as u64)
                            .unwrap_or(0);
                        key_down = false;

                        if suppress_deactivate {
                            suppress_deactivate = false;
                            info!(held_ms, "hotkey release after second tap (ignored)");
                            continue;
                        }

                        if held_ms >= threshold_ms {
                            if pipeline.is_listening() {
                                info!(held_ms, "hotkey hold release → Stop");
                                pipeline.request_stop();
                            }
                        } else {
                            info!(held_ms, "hotkey tap release — stay listening until second tap");
                            pipeline.note_tap_release();
                        }
                    }
                    None => {
                        warn!("Deactivated stream ended");
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_trigger_normalizes_default() {
        assert_eq!(to_portal_trigger("Ctrl+Alt+Space"), "CTRL+ALT+space");
        assert_eq!(to_portal_trigger("CTRL+SHIFT+D"), "CTRL+SHIFT+d");
    }

    #[test]
    fn paste_last_fires_on_press_not_hold_or_repeat() {
        assert!(paste_last_should_fire(false));
        assert!(!paste_last_should_fire(true));
        let cfg = crate::config::NaradConfig::default();
        assert_eq!(cfg.activation.dictate_hotkey, "Ctrl+Alt+Space");
        assert_eq!(cfg.activation.paste_last_hotkey, "Ctrl+Alt+V");
        assert_eq!(cfg.activation.undo_last_hotkey, "Ctrl+Alt+Z");
        assert_eq!(
            to_portal_trigger(&cfg.activation.undo_last_hotkey),
            "CTRL+ALT+z"
        );
        assert_eq!(
            to_portal_trigger(&cfg.activation.paste_last_hotkey),
            "CTRL+ALT+v"
        );
        assert_ne!(
            to_portal_trigger(&cfg.activation.paste_last_hotkey),
            to_portal_trigger(&cfg.activation.dictate_hotkey)
        );
    }
}
