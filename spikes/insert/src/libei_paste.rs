//! Paste Ctrl+V (or Ctrl+Shift+V) via libei on an EIS socket from the portal.

use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Context;
use reis::{
    ei,
    handshake::ei_handshake_blocking,
    PendingRequestResult,
    Interface,
};

/// Linux evdev key codes; libei `keyboard.key` expects `keycode - 8` (see reis `type-text` example).
const KEY_LEFTCTRL: u32 = 29 - 8;
const KEY_LEFTSHIFT: u32 = 42 - 8;
const KEY_V: u32 = 47 - 8;

struct DeviceData {
    keyboard: Option<ei::Keyboard>,
}

/// Inject a paste chord into the focused app using a libei sender context.
pub fn paste_ctrl_v(stream: UnixStream, terminal: bool) -> anyhow::Result<()> {
    let context = ei::Context::new(stream).context("ei::Context::new")?;
    let _handshake = context.handshake();
    context.flush()?;
    let handshake =
        ei_handshake_blocking(&context, "spike-insert", ei::handshake::ContextType::Sender)
            .context("libei handshake")?;

    let mut last_serial = handshake.serial;
    let mut sequence = 0u32;
    let mut keyboard_masks: HashMap<ei::Seat, u64> = HashMap::new();
    let mut devices: HashMap<ei::Device, DeviceData> = HashMap::new();
    let deadline = Instant::now() + Duration::from_secs(15);

    while Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
        context.read()?;
        while let Some(result) = context.pending_event() {
            let event = match result {
                PendingRequestResult::Request(request) => request,
                PendingRequestResult::ParseError(err) => {
                    anyhow::bail!("libei parse error: {err}");
                }
                PendingRequestResult::InvalidObject(_) => continue,
            };

            match event {
                ei::Event::Connection(_connection, request) => {
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
                        let data = devices.get_mut(&device).expect("unknown device");
                        if object.interface() == ei::Keyboard::NAME {
                            data.keyboard = object.clone().downcast();
                        }
                    }
                    ei::device::Event::Done => {
                        if let Some(data) = devices.get(&device) {
                            if data.keyboard.is_some() {
                                emit_paste(
                                    &context,
                                    &device,
                                    data.keyboard.as_ref().unwrap(),
                                    &mut last_serial,
                                    &mut sequence,
                                    terminal,
                                )?;
                                return Ok(());
                            }
                        }
                    }
                    ei::device::Event::Resumed { serial } => {
                        last_serial = serial;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        context.flush()?;
    }

    anyhow::bail!("timed out waiting for libei keyboard device (15s)");
}

fn emit_paste(
    context: &ei::Context,
    device: &ei::Device,
    keyboard: &ei::Keyboard,
    last_serial: &mut u32,
    sequence: &mut u32,
    terminal: bool,
) -> anyhow::Result<()> {
    device.start_emulating(*sequence, *last_serial);
    *sequence += 1;

    if terminal {
        key(keyboard, KEY_LEFTSHIFT, true);
    }
    key(keyboard, KEY_LEFTCTRL, true);
    key(keyboard, KEY_V, true);
    key(keyboard, KEY_V, false);
    key(keyboard, KEY_LEFTCTRL, false);
    if terminal {
        key(keyboard, KEY_LEFTSHIFT, false);
    }

    device.frame(*last_serial, 0);
    device.stop_emulating(*last_serial);
    context.flush()?;
    Ok(())
}

fn key(keyboard: &ei::Keyboard, code: u32, press: bool) {
    let state = if press {
        ei::keyboard::KeyState::Press
    } else {
        ei::keyboard::KeyState::Released
    };
    keyboard.key(code, state);
}
