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

/// Backspace via the same libei session, with portal NotifyKeyboardKeycode fallback.
async fn backspace_chord(
    remote: &RemoteDesktop,
    session: &ashpd::desktop::Session<RemoteDesktop>,
    count: usize,
) -> Result<()> {
    match remote
        .connect_to_eis(session, ConnectToEISOptions::default())
        .await
    {
        Ok(fd) => {
            let stream = UnixStream::from(OwnedFd::from(fd));
            tokio::task::spawn_blocking(move || libei_backspaces(stream, count))
                .await
                .context("libei join")??;
            Ok(())
        }
        Err(err) => {
            warn!(error = %err, "ConnectToEIS failed; falling back to NotifyKeyboardKeycode");
            portal_backspaces(remote, session, count).await
        }
    }
}

const KEY_LEFTCTRL: i32 = 29;
const KEY_LEFTSHIFT: i32 = 42;
const KEY_V: i32 = 47;
/// Linux evdev `KEY_BACKSPACE`. One press+release deletes one Unicode scalar.
pub(crate) const KEY_BACKSPACE: i32 = 14;

async fn portal_key(
    remote: &RemoteDesktop,
    session: &ashpd::desktop::Session<RemoteDesktop>,
    keycode: i32,
    state: KeyState,
) -> Result<()> {
    remote
        .notify_keyboard_keycode(
            session,
            keycode,
            state,
            NotifyKeyboardKeycodeOptions::default(),
        )
        .await
        .context("NotifyKeyboardKeycode")?;
    Ok(())
}

async fn portal_paste(
    remote: &RemoteDesktop,
    session: &ashpd::desktop::Session<RemoteDesktop>,
    terminal: bool,
) -> Result<()> {
    if terminal {
        portal_key(remote, session, KEY_LEFTSHIFT, KeyState::Pressed).await?;
    }
    portal_key(remote, session, KEY_LEFTCTRL, KeyState::Pressed).await?;
    portal_key(remote, session, KEY_V, KeyState::Pressed).await?;
    portal_key(remote, session, KEY_V, KeyState::Released).await?;
    portal_key(remote, session, KEY_LEFTCTRL, KeyState::Released).await?;
    if terminal {
        portal_key(remote, session, KEY_LEFTSHIFT, KeyState::Released).await?;
    }
    Ok(())
}

async fn portal_backspaces(
    remote: &RemoteDesktop,
    session: &ashpd::desktop::Session<RemoteDesktop>,
    count: usize,
) -> Result<()> {
    for _ in 0..count {
        portal_key(remote, session, KEY_BACKSPACE, KeyState::Pressed).await?;
        portal_key(remote, session, KEY_BACKSPACE, KeyState::Released).await?;
    }
    Ok(())
}

fn libei_paste(stream: UnixStream, terminal: bool) -> Result<()> {
    const EI_CTRL: u32 = 29 - 8;
    const EI_SHIFT: u32 = 42 - 8;
    const EI_V: u32 = 47 - 8;
    libei_with_keyboard(stream, move |kb| {
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
    })
}

fn libei_backspaces(stream: UnixStream, count: usize) -> Result<()> {
    const EI_BACKSPACE: u32 = 14 - 8;
    libei_with_keyboard(stream, move |kb| {
        for _ in 0..count {
            kb.key(EI_BACKSPACE, ei::keyboard::KeyState::Press);
            kb.key(EI_BACKSPACE, ei::keyboard::KeyState::Released);
        }
    })
}

/// Same libei keyboard session paste uses. `emit` runs once the keyboard is ready.
fn libei_with_keyboard(stream: UnixStream, emit: impl FnOnce(&ei::Keyboard)) -> Result<()> {
    let mut emit = Some(emit);
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
                                if let Some(emit) = emit.take() {
                                    emit(kb);
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

// Per-task paste record so parallel tests do not share one portal stand-in.
#[cfg(test)]
tokio::task_local! {
    static TEST_INSERT_LOG: std::sync::Arc<std::sync::Mutex<Vec<String>>>;
    static TEST_INSERT_TERMINAL: std::sync::Arc<std::sync::Mutex<Vec<bool>>>;
    static TEST_FORCE_COPY: ();
    pub(crate) static TEST_PRIMARY_SELECTION: std::sync::Arc<std::sync::Mutex<Option<String>>>;
}

#[cfg(test)]
thread_local! {
    static TEST_PRIMARY_SELECTION_FALLBACK: std::sync::Arc<std::sync::Mutex<Option<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
}

/// Record paste text for `fut` and skip the portal.
#[cfg(test)]
pub(crate) async fn with_test_inserts<T>(
    log: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    fut: impl std::future::Future<Output = T>,
) -> T {
    TEST_INSERT_LOG.scope(log, fut).await
}

/// Record paste text and the terminal-chord flag, and force the copied outcome.
#[cfg(test)]
pub(crate) async fn with_test_copied_inserts<T>(
    log: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    chords: std::sync::Arc<std::sync::Mutex<Vec<bool>>>,
    fut: impl std::future::Future<Output = T>,
) -> T {
    TEST_INSERT_LOG
        .scope(
            log,
            TEST_INSERT_TERMINAL.scope(chords, TEST_FORCE_COPY.scope((), fut)),
        )
        .await
}

/// Mock primary selection during `fut`.
#[cfg(test)]
pub(crate) async fn with_test_primary_selection<T>(
    selection: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    fut: impl std::future::Future<Output = T>,
) -> T {
    TEST_PRIMARY_SELECTION.scope(selection, fut).await
}

/// Set mock primary selection for synchronous thread contexts.
#[cfg(test)]
#[allow(dead_code)]
pub fn set_test_primary_selection(text: Option<String>) {
    TEST_PRIMARY_SELECTION_FALLBACK.with(|cell| {
        if let Ok(mut lock) = cell.lock() {
            *lock = text;
        }
    });
}

// Per-task Backspace record. Tests must not open the real keyboard portal.
#[cfg(test)]
tokio::task_local! {
    static TEST_BACKSPACE_LOG: std::sync::Arc<std::sync::Mutex<Vec<i32>>>;
}

/// Reads current text selection via `wl-paste` primary clipboard (FR-INS-6).
pub fn read_primary_selection() -> Option<String> {
    #[cfg(test)]
    {
        if let Ok(arc) = TEST_PRIMARY_SELECTION.try_with(|s| s.clone()) {
            if let Ok(guard) = arc.lock() {
                return guard.as_deref().and_then(|s| {
                    let trimmed = s.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                });
            }
        }
        let fallback = TEST_PRIMARY_SELECTION_FALLBACK.with(|cell| {
            cell.lock().ok().and_then(|guard| guard.clone())
        });
        if let Some(text) = fallback {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return None;
            } else {
                return Some(trimmed.to_string());
            }
        }
    }

    let output = std::process::Command::new("wl-paste")
        .args(["-n", "--primary", "-t", "text/plain"])
        .output();

    let output = match output {
        Ok(out) if out.status.success() => Some(out),
        _ => std::process::Command::new("wl-paste")
            .args(["-n", "-t", "text/plain"])
            .output()
            .ok()
            .filter(|out| out.status.success()),
    }?;

    let text = String::from_utf8(output.stdout).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Record Backspaces for `fut` and skip the portal.
#[cfg(test)]
pub(crate) async fn with_test_backspaces<T>(
    log: std::sync::Arc<std::sync::Mutex<Vec<i32>>>,
    fut: impl std::future::Future<Output = T>,
) -> T {
    TEST_BACKSPACE_LOG.scope(log, fut).await
}

/// `Ok(true)` when this build recorded the keys and must not open the portal.
fn skip_backspaces_for_test(count: usize) -> Result<bool> {
    #[cfg(test)]
    {
        return match TEST_BACKSPACE_LOG.try_with(|log| {
            log.lock()
                .expect("test backspace entries")
                .extend(std::iter::repeat(KEY_BACKSPACE).take(count));
        }) {
            Ok(()) => Ok(true),
            Err(_) => Err(anyhow!("backspace test log is not set")),
        };
    }
    #[cfg(not(test))]
    {
        let _ = count;
        Ok(false)
    }
}

/// Send `count` Backspace keypresses. Callers pass `text.chars().count()` (Unicode scalars).
pub async fn send_backspaces(count: usize) -> Result<()> {
    if skip_backspaces_for_test(count)? {
        return Ok(());
    }

    if count == 0 {
        return Ok(());
    }

    let (remote, session, _) = open_remote_desktop_session(false)
        .await
        .context("RemoteDesktop for backspace")?;
    backspace_chord(&remote, &session, count).await?;
    info!(count, "backspace sequence sent");
    Ok(())
}

/// GTK app ids and wm_class names that paste with Ctrl+Shift+V (FR-INS-2).
const TERMINAL_APPS: &[&str] = &[
    "org.gnome.Terminal",
    "gnome-terminal",
    "org.gnome.Console",
    "kgx",
    "org.gnome.Ptyxis",
    "ptyxis",
];

/// True only for GNOME Terminal, Console, and Ptyxis. Unknown apps stay Ctrl+V.
pub(crate) fn uses_terminal_paste(app_id: Option<&str>) -> bool {
    match app_id {
        Some(id) => TERMINAL_APPS.iter().any(|known| *known == id),
        None => false,
    }
}

/// Chord shown on the copied pill for the paste that just failed.
pub(crate) fn paste_hint(terminal: bool) -> &'static str {
    if terminal {
        "Ctrl+Shift+V"
    } else {
        "Ctrl+V"
    }
}

/// Insert `text` at the focused cursor; on failure leave it on the clipboard (FR-INS-1/2/3).
///
/// `terminal` selects Ctrl+Shift+V for GNOME Terminal, Console, and Ptyxis, otherwise Ctrl+V.
pub async fn insert_text(text: &str, terminal: bool) -> InsertOutcome {
    #[cfg(test)]
    if let Ok(log) = TEST_INSERT_LOG.try_with(|log| log.clone()) {
        log.lock().expect("test insert entries").push(text.to_string());
        let _ = TEST_INSERT_TERMINAL.try_with(|flags| {
            flags.lock().expect("test paste chords").push(terminal);
        });
        let force_copy = TEST_FORCE_COPY.try_with(|_| ()).is_ok();
        return if text.is_empty() || force_copy {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn only_the_three_vte_apps_use_shift_paste() {
        assert!(uses_terminal_paste(Some("org.gnome.Terminal")));
        assert!(uses_terminal_paste(Some("gnome-terminal")));
        assert!(uses_terminal_paste(Some("org.gnome.Console")));
        assert!(uses_terminal_paste(Some("kgx")));
        assert!(uses_terminal_paste(Some("org.gnome.Ptyxis")));
        assert!(uses_terminal_paste(Some("ptyxis")));
        assert!(!uses_terminal_paste(Some("org.gnome.gedit")));
        assert!(!uses_terminal_paste(Some("code")));
        assert!(!uses_terminal_paste(None));
        assert_eq!(paste_hint(true), "Ctrl+Shift+V");
        assert_eq!(paste_hint(false), "Ctrl+V");
    }

    #[tokio::test]
    async fn backspaces_match_unicode_scalars() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let text = "café👍";
        with_test_backspaces(Arc::clone(&log), send_backspaces(text.chars().count()))
            .await
            .expect("backspaces");
        let keys = log.lock().expect("log").clone();
        assert_eq!(keys.len(), text.chars().count());
        assert!(keys.iter().all(|key| *key == KEY_BACKSPACE));
        assert_ne!(text.chars().count(), text.len());
    }

    #[tokio::test]
    async fn primary_selection_capture_and_trimming() {
        let mock = Arc::new(Mutex::new(Some("  selected text  ".to_string())));
        let captured = with_test_primary_selection(Arc::clone(&mock), async {
            read_primary_selection()
        })
        .await;
        assert_eq!(captured.as_deref(), Some("selected text"));

        let empty_mock = Arc::new(Mutex::new(Some("   \n\t ".to_string())));
        let empty_captured = with_test_primary_selection(empty_mock, async {
            read_primary_selection()
        })
        .await;
        assert_eq!(empty_captured, None);

        let none_mock = Arc::new(Mutex::new(None));
        let none_captured = with_test_primary_selection(none_mock, async {
            read_primary_selection()
        })
        .await;
        assert_eq!(none_captured, None);
    }
}
