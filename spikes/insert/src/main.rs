//! Phase 0 spike: clipboard + paste into the focused app via xdg-desktop-portal.

mod libei_paste;
mod portal_paste;

use std::fs;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use anyhow::Context;
use ashpd::desktop::clipboard::{Clipboard, RequestClipboardOptions, SetSelectionOptions};
use ashpd::desktop::remote_desktop::{
    ConnectToEISOptions, DeviceType, RemoteDesktop, SelectDevicesOptions, StartOptions,
};
use ashpd::desktop::{CreateSessionOptions, PersistMode};
use clap::Parser;
use enumflags2::BitFlags;
use futures_util::StreamExt;
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum PasteMethod {
    /// `RemoteDesktop.ConnectToEIS` + libei keyboard (default).
    Libei,
    /// `RemoteDesktop.NotifyKeyboardKeycode` (portal D-Bus; no reis client).
    Portal,
}

#[derive(Parser, Debug)]
#[command(about = "Phase 0 insert spike: set clipboard and paste into focused app")]
struct Args {
    /// Text to insert (clipboard + paste).
    #[arg(short, long, default_value = "Hello from spike-insert")]
    text: String,

    /// Seconds to wait before paste so you can focus a text field.
    #[arg(long, default_value_t = 3)]
    delay_secs: u64,

    /// Use Ctrl+Shift+V (GNOME Terminal and other VTE apps).
    #[arg(long)]
    terminal: bool,

    #[arg(long, value_enum, default_value_t = PasteMethod::Libei)]
    paste: PasteMethod,

    /// Ignore saved restore token and force a new portal permission dialog.
    #[arg(long)]
    forget_token: bool,
}

fn restore_token_path() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share"))
        })
        .unwrap_or_else(|| PathBuf::from(".local/share"));
    base.join("narad/spike-insert/restore_token")
}

fn read_restore_token() -> Option<String> {
    let path = restore_token_path();
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

fn write_restore_token(token: &str) -> anyhow::Result<()> {
    let path = restore_token_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    info!(path = %path.display(), "saved RemoteDesktop restore token");
    fs::write(&path, token)?;
    Ok(())
}

fn clear_restore_token() -> anyhow::Result<()> {
    let path = restore_token_path();
    if path.exists() {
        info!(path = %path.display(), "removed restore token");
        fs::remove_file(&path)?;
    }
    Ok(())
}

async fn spawn_clipboard_transfer_handler(text: String) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let clipboard = match Clipboard::new().await {
            Ok(c) => c,
            Err(err) => {
                warn!("clipboard portal: {err}");
                return;
            }
        };

        let transfers = match clipboard
            .receive_selection_transfer::<RemoteDesktop>()
            .await
        {
            Ok(stream) => stream,
            Err(err) => {
                warn!("clipboard SelectionTransfer stream: {err}");
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

/// Advertise clipboard content after the session is started with clipboard enabled.
async fn advertise_clipboard(
    clipboard: &Clipboard,
    session: &ashpd::desktop::Session<RemoteDesktop>,
    text: &str,
) -> anyhow::Result<()> {
    let _writer = spawn_clipboard_transfer_handler(text.to_owned());

    clipboard
        .set_selection(
            session,
            SetSelectionOptions::default().set_mime_types(&["text/plain"]),
        )
        .await
        .context("Clipboard.SetSelection")?;

    info!("clipboard selection advertised (text/plain)");
    Ok(())
}

/// Create → SelectDevices → RequestClipboard → Start.
///
/// Portal docs: RequestClipboard must run *before* Start; SetSelection only after.
async fn open_remote_desktop_session(
    forget_token: bool,
) -> anyhow::Result<(
    RemoteDesktop,
    ashpd::desktop::Session<RemoteDesktop>,
    bool,
)> {
    if forget_token {
        clear_restore_token()?;
    }

    let remote = RemoteDesktop::new()
        .await
        .context("connect to org.freedesktop.portal.RemoteDesktop")?;

    let session = remote
        .create_session(CreateSessionOptions::default())
        .await
        .context("RemoteDesktop.CreateSession")?;

    let mut select = SelectDevicesOptions::default()
        .set_devices(BitFlags::from(DeviceType::Keyboard))
        .set_persist_mode(PersistMode::ExplicitlyRevoked);

    if let Some(token) = read_restore_token() {
        info!("using saved restore token");
        select = select.set_restore_token(token.as_str());
    }

    remote
        .select_devices(&session, select)
        .await
        .context("RemoteDesktop.SelectDevices")?
        .response()
        .context("SelectDevices response")?;

    let clipboard = Clipboard::new()
        .await
        .context("connect to org.freedesktop.portal.Clipboard")?;
    clipboard
        .request(&session, RequestClipboardOptions::default())
        .await
        .context(
            "Clipboard.RequestClipboard (must be called before RemoteDesktop.Start)",
        )?;
    info!("Clipboard.RequestClipboard OK (before Start)");

    let started = remote
        .start(&session, None, StartOptions::default())
        .await
        .context("RemoteDesktop.Start")?
        .response()
        .context("Start response")?;

    let clipboard_enabled = started.is_clipboard_enabled();
    info!(
        devices = ?started.devices(),
        clipboard_enabled,
        "RemoteDesktop session started"
    );

    if let Some(token) = started.restore_token() {
        write_restore_token(token)?;
    } else {
        warn!("portal did not return a restore_token (persistence may be unavailable)");
    }

    Ok((remote, session, clipboard_enabled))
}

async fn paste(
    remote: &RemoteDesktop,
    session: &ashpd::desktop::Session<RemoteDesktop>,
    method: PasteMethod,
    terminal: bool,
) -> anyhow::Result<()> {
    match method {
        PasteMethod::Portal => {
            portal_paste::paste_ctrl_v_portal(remote, session, terminal)
                .await
                .context("NotifyKeyboardKeycode paste")?;
            info!("sent paste chord via portal NotifyKeyboardKeycode");
        }
        PasteMethod::Libei => {
            let fd = remote
                .connect_to_eis(session, ConnectToEISOptions::default())
                .await
                .context("RemoteDesktop.ConnectToEIS (requires portal v2+)")?;
            let stream = UnixStream::from(OwnedFd::from(fd));
            tokio::task::spawn_blocking(move || libei_paste::paste_ctrl_v(stream, terminal))
                .await
                .context("libei paste task")??;
            info!("sent paste chord via libei (ConnectToEIS)");
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    info!(
        delay_secs = args.delay_secs,
        terminal = args.terminal,
        paste = ?args.paste,
        "focus a text field; paste will run after the delay"
    );

    // Old restore tokens may predate clipboard enablement; retry once fresh.
    let (remote, session) = match open_remote_desktop_session(args.forget_token).await {
        Ok((r, s, true)) => (r, s),
        Ok((_, _, false)) if !args.forget_token => {
            warn!(
                "restored session has clipboard_enabled=false; clearing token and retrying once"
            );
            clear_restore_token()?;
            let (r, s, enabled) = open_remote_desktop_session(true).await?;
            if !enabled {
                anyhow::bail!(
                    "clipboard still disabled after fresh Start — check the portal dialog"
                );
            }
            (r, s)
        }
        Ok((_, _, false)) => {
            anyhow::bail!("clipboard_enabled=false after --forget-token Start");
        }
        Err(err) => return Err(err),
    };

    let clipboard = Clipboard::new()
        .await
        .context("connect to org.freedesktop.portal.Clipboard")?;
    advertise_clipboard(&clipboard, &session, &args.text).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(args.delay_secs)).await;

    paste(&remote, &session, args.paste, args.terminal).await?;

    info!("done — verify the focused app received: {:?}", args.text);
    Ok(())
}
