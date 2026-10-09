//! Fallback paste via RemoteDesktop `NotifyKeyboardKeycode` (portal D-Bus API).

use ashpd::desktop::remote_desktop::{KeyState, NotifyKeyboardKeycodeOptions, RemoteDesktop};
use ashpd::desktop::Session;

/// Linux evdev key codes for `NotifyKeyboardKeycode`.
const KEY_LEFTCTRL: i32 = 29;
const KEY_LEFTSHIFT: i32 = 42;
const KEY_V: i32 = 47;

/// Paste using portal keyboard notifications (no explicit libei client).
pub async fn paste_ctrl_v_portal(
    remote: &RemoteDesktop,
    session: &Session<RemoteDesktop>,
    terminal: bool,
) -> ashpd::Result<()> {
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

async fn key(
    remote: &RemoteDesktop,
    session: &Session<RemoteDesktop>,
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
