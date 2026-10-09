//! Clipboard + paste via RemoteDesktop / libei (FR-INS-1/2/3).

use std::fs;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use ashpd::desktop::clipboard::{Clipboard, RequestClipboardOptions, SetSelectionOptions};
use ashpd::desktop::remote_desktop::{
    ConnectToEISOptions, DeviceType, KeyState, NotifyKeyboardKeycodeOptions, RemoteDesktop,
    SelectDevicesOptions, StartOptions,
};
use ashpd::desktop::{CreateSessionOptions, PersistMode};
use enumflags2::BitFlags;
use futures_util::StreamExt;
use reis::{ei, handshake::ei_handshake_blocking, Interface, PendingRequestResult};
use tracing::{info, warn};

/// Outcome of an insert attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertOutcome {
    /// Text pasted into the focused field.
    Inserted,
    /// Paste failed or unavailable; text left on clipboard (FR-INS-3).
    Copied,
}

fn restore_token_path() -> PathBuf {
    crate::paths::resolve_data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("insert")
        .join("restore_token")
}

fn read_restore_token() -> Option<String> {
    fs::read_to_string(restore_token_path())
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

fn write_restore_token(token: &str) -> Result<()> {
    let path = restore_token_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, token)?;
    Ok(())
}

fn clear_restore_token() -> Result<()> {
    let path = restore_token_path();
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

fn spawn_clipboard_transfer_handler(text: String) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let clipboard = match Clipboard::new().await {
            Ok(c) => c,
            Err(err) => {
                warn!("clipboard portal: {err}");
                return;
            }
        };
        let transfers = match clipboard.receive_selection_transfer::<RemoteDesktop>().await {
            Ok(stream) => stream,
            Err(err) => {
                warn!("SelectionTransfer stream: {err}");
                return;
            }
        };
        futures_util::pin_mut!(transfers);
        while let Some((session, mime_type, serial)) = transfers.next().await {
            if mime_type != "text/plain" {
                let _ = clipboard
                    .selection_write_done(&session, serial, false)
                    .await;
                continue;
            }
            let success = match clipboard.selection_write(&session, serial).await {
                Ok(fd) => {
                    let mut file = std::fs::File::from(OwnedFd::from(fd));
                    std::io::Write::write_all(&mut file, text.as_bytes()).is_ok()
                }
                Err(err) => {
                    warn!("SelectionWrite failed: {err}");
                    false
                }
            };
            let _ = clipboard
                .selection_write_done(&session, serial, success)
                .await;
        }
    })
}

/// CreateSession → SelectDevices → RequestClipboard → Start (order required by portal).
async fn open_remote_desktop_session(
    forget_token: bool,
) -> Result<(RemoteDesktop, ashpd::desktop::Session<RemoteDesktop>, bool)> {
    if forget_token {
        clear_restore_token()?;
    }

    let remote = RemoteDesktop::new()
        .await
        .context("connect RemoteDesktop portal")?;
    let session = remote
        .create_session(CreateSessionOptions::default())
        .await
        .context("RemoteDesktop.CreateSession")?;

    let mut select = SelectDevicesOptions::default()
        .set_devices(BitFlags::from(DeviceType::Keyboard))
        .set_persist_mode(PersistMode::ExplicitlyRevoked);
    if let Some(token) = read_restore_token() {
        select = select.set_restore_token(token.as_str());
    }

    remote
        .select_devices(&session, select)
        .await
        .context("SelectDevices")?
        .response()
        .context("SelectDevices response")?;

    let clipboard = Clipboard::new().await.context("Clipboard portal")?;
    clipboard
        .request(&session, RequestClipboardOptions::default())
        .await
        .context("RequestClipboard (must be before Start)")?;

    let started = remote
        .start(&session, None, StartOptions::default())
        .await
        .context("RemoteDesktop.Start")?
        .response()
        .context("Start response")?;

    let clipboard_enabled = started.is_clipboard_enabled();
    if let Some(token) = started.restore_token() {
        let _ = write_restore_token(token);
    }
    Ok((remote, session, clipboard_enabled))
}

async fn advertise_clipboard(
    clipboard: &Clipboard,
    session: &ashpd::desktop::Session<RemoteDesktop>,
    text: &str,
) -> Result<()> {
    let _writer = spawn_clipboard_transfer_handler(text.to_owned());
    clipboard
        .set_selection(
            session,
            SetSelectionOptions::default().set_mime_types(&["text/plain"]),
        )
        .await
        .context("Clipboard.SetSelection")?;
    Ok(())
}

/// Paste via libei (preferred) with portal NotifyKeyboardKeycode fallback.
async fn paste_chord(
    remote: &RemoteDesktop,
    session: &ashpd::desktop::Session<RemoteDesktop>,
    terminal: bool,
) -> Result<()> {
    match remote
        .connect_to_eis(session, ConnectToEISOptions::default())
        .await
    {
        Ok(fd) => {
            let stream = UnixStream::from(OwnedFd::from(fd));
            tokio::task::spawn_blocking(move || libei_paste(stream, terminal))
                .await
                .context("libei join")??;
            Ok(())
        }
        Err(err) => {
            warn!(error = %err, "ConnectToEIS failed; falling back to NotifyKeyboardKeycode");
            portal_paste(remote, session, terminal).await
        }
    }
}

const KEY_LEFTCTRL: i32 = 29;
const KEY_LEFTSHIFT: i32 = 42;
const KEY_V: i32 = 47;

async fn portal_paste(
    remote: &RemoteDesktop,
    session: &ashpd::desktop::Session<RemoteDesktop>,
    terminal: bool,
) -> Result<()> {
    async fn key(
        remote: &RemoteDesktop,
        session: &ashpd::desktop::Session<RemoteDesktop>,
        keycode: i32,
        state: KeyState,
    ) -> ashpd::Result<()> {
        remote
            .notify_keyboard_keycode(
                session,
                keycode,
                state,
                NotifyKeyboardKeycodeOptions::default(),
            )
            .await
    }
    if terminal {
        key(remote, session, KEY_LEFTSHIFT, KeyState::Pressed).await?;
    }
    key(remote, session, KEY_LEFTCTRL, KeyState::Pressed).await?;
    key(remote, session, KEY_V, KeyState::Pressed).await?;
    key(remote, session, KEY_V, KeyState::Released).await?;
    key(remote, session, KEY_LEFTCTRL, KeyState::Released).await?;
    if terminal {
        key(remote, session, KEY_LEFTSHIFT, KeyState::Released).await?;
    }
    Ok(())
}

fn libei_paste(stream: UnixStream, terminal: bool) -> Result<()> {
    const EI_CTRL: u32 = 29 - 8;
    const EI_SHIFT: u32 = 42 - 8;
    const EI_V: u32 = 47 - 8;

    struct DeviceData {
        keyboard: Option<ei::Keyboard>,
    }

    let context = ei::Context::new(stream).context("ei::Context")?;
    let _ = context.handshake();
    context.flush()?;
    let handshake =
        ei_handshake_blocking(&context, "naradd", ei::handshake::ContextType::Sender)
            .context("libei handshake")?;

    let mut last_serial = handshake.serial;
    let mut sequence = 0u32;
    let mut keyboard_masks: HashMap<ei::Seat, u64> = HashMap::new();
    let mut devices: HashMap<ei::Device, DeviceData> = HashMap::new();
    let deadline = Instant::now() + Duration::from_secs(10);

    while Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
        context.read()?;
        while let Some(result) = context.pending_event() {
            let event = match result {
                PendingRequestResult::Request(request) => request,
                PendingRequestResult::ParseError(err) => bail!("libei parse: {err}"),
                PendingRequestResult::InvalidObject(_) => continue,
            };
            match event {
                ei::Event::Connection(_c, request) => {
                    if let ei::connection::Event::Ping { ping } = request {
                        ping.done(0);
                    }
                }
                ei::Event::Seat(seat, request) => match request {
                    ei::seat::Event::Capability { mask, interface } => {
                        if interface == ei::Keyboard::NAME {
                            keyboard_masks.insert(seat, mask);
                        }
                    }
                    ei::seat::Event::Done => {
                        if let Some(mask) = keyboard_masks.remove(&seat) {
                            seat.bind(mask);
                        }
                    }
                    ei::seat::Event::Device { device } => {
                        devices.insert(device, DeviceData { keyboard: None });
                    }
                    _ => {}
                },
                ei::Event::Device(device, request) => match request {
                    ei::device::Event::Interface { object } => {
                        if let Some(data) = devices.get_mut(&device) {
                            if object.interface() == ei::Keyboard::NAME {
                                data.keyboard = object.clone().downcast();
                            }
                        }
                    }
                    ei::device::Event::Done => {
                        if let Some(data) = devices.get(&device) {
                            if let Some(ref kb) = data.keyboard {
                                device.start_emulating(sequence, last_serial);
                                sequence = sequence.wrapping_add(1);
                                let _ = sequence;
                                let press = |keyboard: &ei::Keyboard, code: u32, down: bool| {
                                    let state = if down {
                                        ei::keyboard::KeyState::Press
                                    } else {
                                        ei::keyboard::KeyState::Released
                                    };
                                    keyboard.key(code, state);
                                };
                                if terminal {
                                    press(kb, EI_SHIFT, true);
                                }
                                press(kb, EI_CTRL, true);
                                press(kb, EI_V, true);
                                press(kb, EI_V, false);
                                press(kb, EI_CTRL, false);
                                if terminal {
                                    press(kb, EI_SHIFT, false);
                                }
                                device.frame(last_serial, 0);
                                device.stop_emulating(last_serial);
                                context.flush()?;
                                return Ok(());
                            }
                        }
                    }
                    ei::device::Event::Resumed { serial } => last_serial = serial,
                    _ => {}
                },
                _ => {}
            }
        }
        context.flush()?;
    }
    Err(anyhow!("timed out waiting for libei keyboard"))
}

fn copy_via_wl_copy(text: &str) -> Result<()> {
    let status = std::process::Command::new("wl-copy")
        .arg("--")
        .arg(text)
        .status()
        .context("spawn wl-copy")?;
    if !status.success() {
        bail!("wl-copy exited {status}");
    }
    Ok(())
}

/// Snapshot clipboard via `wl-paste`. Never logs contents (NFR-S-4).
fn snapshot_clipboard_wl() -> Option<String> {
    let output = std::process::Command::new("wl-paste")
        .args(["-n", "-t", "text/plain"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

/// Snapshot current clipboard text before SetSelection (FR-INS-2). Never logs contents (NFR-S-4).
async fn snapshot_clipboard(
    clipboard: &Clipboard,
    session: &ashpd::desktop::Session<RemoteDesktop>,
) -> Option<String> {
    match clipboard.selection_read(session, "text/plain").await {
        Ok(fd) => {
            let mut file = std::fs::File::from(OwnedFd::from(fd));
            let mut buf = Vec::new();
            if std::io::Read::read_to_end(&mut file, &mut buf).is_ok() {
                if let Ok(s) = String::from_utf8(buf) {
                    return Some(s);
                }
            }
        }
        Err(_) => {}
    }
    snapshot_clipboard_wl()
}

/// Restore prior clipboard after a successful paste. Errors are non-fatal.
fn restore_clipboard_text(text: &str) -> Result<()> {
    copy_via_wl_copy(text)
}

/// Test stand-in for the portal paste. `None` uses the real inserter.
#[cfg(test)]
static TEST_INSERTS: std::sync::Mutex<Option<std::sync::Arc<std::sync::Mutex<Vec<String>>>>> =
    std::sync::Mutex::new(None);

/// Record paste text and skip the portal. Only the naradd test binary calls this.
#[cfg(test)]
pub(crate) fn set_test_insert_log(log: Option<std::sync::Arc<std::sync::Mutex<Vec<String>>>>) {
    *TEST_INSERTS.lock().expect("test insert log") = log;
}

/// Insert `text` at the focused cursor; on failure leave it on the clipboard (FR-INS-1/2/3).
///
/// `terminal`: Ctrl+Shift+V when true, else Ctrl+V. Callers keep `false` until Phase 2
/// app context can detect terminals (FR-CTX-1); do not invent detection here.
pub async fn insert_text(text: &str, terminal: bool) -> InsertOutcome {
    #[cfg(test)]
    if let Some(log) = TEST_INSERTS.lock().expect("test insert log").clone() {
        log.lock().expect("test insert entries").push(text.to_string());
        return if text.is_empty() {
            InsertOutcome::Copied
        } else {
            InsertOutcome::Inserted
        };
    }

    if text.is_empty() {
        return InsertOutcome::Copied;
    }

    let session_result = match open_remote_desktop_session(false).await {
        Ok((r, s, true)) => Ok((r, s)),
        Ok((_, _, false)) => {
            warn!("clipboard_enabled=false on restore; retrying with fresh token");
            let _ = clear_restore_token();
            match open_remote_desktop_session(true).await {
                Ok((r, s, true)) => Ok((r, s)),
                Ok((_, _, false)) => Err(anyhow!("clipboard still disabled")),
                Err(e) => Err(e),
            }
        }
        Err(e) => Err(e),
    };

    let (remote, session) = match session_result {
        Ok(pair) => pair,
        Err(err) => {
            warn!(error = %err, "RemoteDesktop unavailable; falling back to wl-copy");
            match copy_via_wl_copy(text) {
                Ok(()) => return InsertOutcome::Copied,
                Err(e) => {
                    warn!(error = %e, "wl-copy also failed");
                    return InsertOutcome::Copied;
                }
            }
        }
    };

    let clipboard = match Clipboard::new().await {
        Ok(c) => c,
        Err(err) => {
            warn!(error = %err, "Clipboard portal failed");
            let _ = copy_via_wl_copy(text);
            return InsertOutcome::Copied;
        }
    };

    // FR-INS-2 / ARCHITECTURE §4: save → set → paste → restore (keep Narad text only for FR-INS-3).
    let prior = snapshot_clipboard(&clipboard, &session).await;

    if let Err(err) = advertise_clipboard(&clipboard, &session, text).await {
        warn!(error = %err, "SetSelection failed");
        let _ = copy_via_wl_copy(text);
        return InsertOutcome::Copied;
    }

    // Brief yield so the transfer handler is ready.
    tokio::time::sleep(Duration::from_millis(50)).await;

    match paste_chord(&remote, &session, terminal).await {
        Ok(()) => {
            info!("paste chord sent");
            // Let the focused app finish reading our selection before restore.
            tokio::time::sleep(Duration::from_millis(100)).await;
            match &prior {
                Some(prev) => match restore_clipboard_text(prev) {
                    Ok(()) => warn!("clipboard restore ok"),
                    Err(err) => warn!(error = %err, "clipboard restore failed"),
                },
                None => warn!("clipboard restore skipped: no prior text snapshot"),
            }
            InsertOutcome::Inserted
        }
        Err(err) => {
            warn!(error = %err, "paste failed; text remains on clipboard");
            InsertOutcome::Copied
        }
    }
}
