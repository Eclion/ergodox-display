//! Live keypress capture through the Oryx raw-HID protocol.
//!
//! The Oryx firmware exposes a raw HID interface (usage page 0xFF60, usage
//! 0x61, 32-byte packets). After the host sends `ORYX_CMD_PAIRING_INIT` the
//! keyboard answers `ORYX_EVT_PAIRING_SUCCESS` and starts streaming
//! `ORYX_EVT_KEYDOWN`/`ORYX_EVT_KEYUP` (payload: col, row) and
//! `ORYX_EVT_LAYER` (payload: highest active layer) packets.

use hidapi::{HidApi, HidDevice};
use serde::Serialize;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

// Match any ZSA keyboard exposing the Oryx raw-HID interface rather than
// pinning a product id: the Ergodox EZ base/shine/glow variants report
// different PIDs (0x4974/0x4975/0x4976).
const VENDOR_ID: u16 = 0x3297; // ZSA Technology Labs
const USAGE_PAGE: u16 = 0xFF60;
const USAGE: u16 = 0x61;
const PACKET_SIZE: usize = 32;

const ORYX_CMD_PAIRING_INIT: u8 = 0x01;
const ORYX_EVT_PAIRING_SUCCESS: u8 = 0x04;
const ORYX_EVT_LAYER: u8 = 0x05;
const ORYX_EVT_KEYDOWN: u8 = 0x06;
const ORYX_EVT_KEYUP: u8 = 0x07;

const EVENT_NAME: &str = "kb-event";

#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum KbEvent {
    Status { connected: bool, detail: String },
    Layer { layer: u8 },
    Key { down: bool, row: u8, col: u8 },
}

pub fn spawn(app: AppHandle) {
    thread::spawn(move || run(app));
}

fn run(app: AppHandle) {
    let mut api = match HidApi::new() {
        Ok(api) => api,
        Err(err) => {
            emit(&app, disconnected(format!("hidapi init failed: {err}")));
            return;
        }
    };
    let mut last_detail = String::new();
    loop {
        match connect(&mut api) {
            Ok(device) => {
                emit(
                    &app,
                    KbEvent::Status {
                        connected: true,
                        detail: "paired".into(),
                    },
                );
                last_detail.clear();
                let err = listen(&app, &device);
                emit(&app, disconnected(err));
            }
            Err(detail) => {
                // Only report state changes, this loop runs every 2 seconds.
                if detail != last_detail {
                    last_detail = detail.clone();
                    emit(&app, disconnected(detail));
                }
            }
        }
        thread::sleep(Duration::from_secs(2));
    }
}

fn disconnected(detail: impl Into<String>) -> KbEvent {
    KbEvent::Status {
        connected: false,
        detail: detail.into(),
    }
}

fn emit(app: &AppHandle, event: KbEvent) {
    let _ = app.emit(EVENT_NAME, &event);
}

fn connect(api: &mut HidApi) -> Result<HidDevice, String> {
    api.refresh_devices().map_err(|e| e.to_string())?;
    let info = api
        .device_list()
        .find(|d| {
            d.vendor_id() == VENDOR_ID && d.usage_page() == USAGE_PAGE && d.usage() == USAGE
        })
        .ok_or_else(|| "keyboard not found".to_string())?;
    let device = info.open_device(api).map_err(|e| {
        format!("cannot open HID device ({e}) — on Linux check the udev rule, see README")
    })?;

    // Pair: first byte is the HID report number (0 = unnumbered).
    let mut cmd = [0u8; PACKET_SIZE + 1];
    cmd[1] = ORYX_CMD_PAIRING_INIT;
    device.write(&cmd).map_err(|e| e.to_string())?;

    let mut buf = [0u8; PACKET_SIZE];
    for _ in 0..5 {
        let n = device
            .read_timeout(&mut buf, 1000)
            .map_err(|e| e.to_string())?;
        if n > 0 && buf[0] == ORYX_EVT_PAIRING_SUCCESS {
            return Ok(device);
        }
    }
    Err("pairing timed out".into())
}

fn listen(app: &AppHandle, device: &HidDevice) -> String {
    let mut buf = [0u8; PACKET_SIZE];
    loop {
        match device.read_timeout(&mut buf, 1000) {
            Ok(0) => continue, // timeout, keep listening
            Ok(_) => match buf[0] {
                ORYX_EVT_LAYER => emit(app, KbEvent::Layer { layer: buf[1] }),
                ORYX_EVT_KEYDOWN | ORYX_EVT_KEYUP => emit(
                    app,
                    KbEvent::Key {
                        down: buf[0] == ORYX_EVT_KEYDOWN,
                        col: buf[1],
                        row: buf[2],
                    },
                ),
                _ => {}
            },
            Err(err) => return err.to_string(),
        }
    }
}
