//! Translate QMK keycodes into the characters the OS actually produces
//! under its active keyboard layout (e.g. AZERTY vs QWERTY).
//!
//! The keyboard sends fixed HID usages; what character they produce is
//! decided by the OS input layout. A background thread resolves, for every
//! printable QMK keycode, the character the current layout emits for it
//! (unshifted for plain keycodes, shifted for the KC_EXLM-style aliases),
//! and pushes a `layout-map` event whenever the result changes — i.e.
//! whenever the user switches input language.
//!
//! Linux (X11): xkbcommon against the server's current keymap and layout
//! group. Windows: MapVirtualKeyEx/ToUnicodeEx against the foreground
//! thread's HKL. Detection is a 2s poll: layout switches are rare and the
//! computation is cheap.

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const EVENT_NAME: &str = "layout-map";

#[derive(Clone, Serialize, PartialEq)]
pub struct LayoutMap {
    pub name: String,
    pub map: HashMap<String, String>,
}

pub struct LayoutMapState(pub Mutex<Option<LayoutMap>>);

/// Printable keys: QMK keycode name, evdev keycode (Linux), scan code set 1
/// (Windows). The OS-specific code derives the layout's character for each.
const KEYS: &[(&str, u32, u16)] = &[
    ("KC_A", 30, 0x1E),
    ("KC_B", 48, 0x30),
    ("KC_C", 46, 0x2E),
    ("KC_D", 32, 0x20),
    ("KC_E", 18, 0x12),
    ("KC_F", 33, 0x21),
    ("KC_G", 34, 0x22),
    ("KC_H", 35, 0x23),
    ("KC_I", 23, 0x17),
    ("KC_J", 36, 0x24),
    ("KC_K", 37, 0x25),
    ("KC_L", 38, 0x26),
    ("KC_M", 50, 0x32),
    ("KC_N", 49, 0x31),
    ("KC_O", 24, 0x18),
    ("KC_P", 25, 0x19),
    ("KC_Q", 16, 0x10),
    ("KC_R", 19, 0x13),
    ("KC_S", 31, 0x1F),
    ("KC_T", 20, 0x14),
    ("KC_U", 22, 0x16),
    ("KC_V", 47, 0x2F),
    ("KC_W", 17, 0x11),
    ("KC_X", 45, 0x2D),
    ("KC_Y", 21, 0x15),
    ("KC_Z", 44, 0x2C),
    ("KC_1", 2, 0x02),
    ("KC_2", 3, 0x03),
    ("KC_3", 4, 0x04),
    ("KC_4", 5, 0x05),
    ("KC_5", 6, 0x06),
    ("KC_6", 7, 0x07),
    ("KC_7", 8, 0x08),
    ("KC_8", 9, 0x09),
    ("KC_9", 10, 0x0A),
    ("KC_0", 11, 0x0B),
    ("KC_MINUS", 12, 0x0C),
    ("KC_EQUAL", 13, 0x0D),
    ("KC_LBRC", 26, 0x1A),
    ("KC_RBRC", 27, 0x1B),
    ("KC_BSLS", 43, 0x2B),
    ("KC_SCLN", 39, 0x27),
    ("KC_QUOTE", 40, 0x28),
    ("KC_GRAVE", 41, 0x29),
    ("KC_COMMA", 51, 0x33),
    ("KC_DOT", 52, 0x34),
    ("KC_SLASH", 53, 0x35),
    ("KC_NONUS_BSLS", 86, 0x56),
];

/// Shift-alias keycodes (Oryx uses these for symbols): label is the shifted
/// character of the underlying key.
const SHIFT_ALIASES: &[(&str, &str)] = &[
    ("KC_EXLM", "KC_1"),
    ("KC_AT", "KC_2"),
    ("KC_HASH", "KC_3"),
    ("KC_DLR", "KC_4"),
    ("KC_PERC", "KC_5"),
    ("KC_CIRC", "KC_6"),
    ("KC_AMPR", "KC_7"),
    ("KC_ASTR", "KC_8"),
    ("KC_LPRN", "KC_9"),
    ("KC_RPRN", "KC_0"),
    ("KC_UNDS", "KC_MINUS"),
    ("KC_PLUS", "KC_EQUAL"),
    ("KC_LCBR", "KC_LBRC"),
    ("KC_RCBR", "KC_RBRC"),
    ("KC_PIPE", "KC_BSLS"),
    ("KC_COLN", "KC_SCLN"),
    ("KC_DQUO", "KC_QUOTE"),
    ("KC_TILD", "KC_GRAVE"),
    ("KC_LABK", "KC_COMMA"),
    ("KC_RABK", "KC_DOT"),
    ("KC_QUES", "KC_SLASH"),
];

pub fn spawn(app: AppHandle) {
    thread::spawn(move || loop {
        if let Some(current) = platform::current_layout() {
            let state = app.state::<LayoutMapState>();
            let mut last = state.0.lock().unwrap();
            if last.as_ref() != Some(&current) {
                *last = Some(current.clone());
                drop(last);
                let _ = app.emit(EVENT_NAME, &current);
            }
        }
        thread::sleep(Duration::from_secs(2));
    });
}

/// Assemble the final keycode -> label map from per-key (unshifted, shifted)
/// characters. A key displays uppercase only when Shift genuinely produces
/// the uppercase of its character (plain letter keys) — not e.g. the AZERTY
/// é key, where Shift produces 2.
fn build_map(chars: HashMap<&'static str, (String, String)>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (kc, (unshifted, shifted)) in &chars {
        if unshifted.is_empty() {
            continue;
        }
        let upper = unshifted.to_uppercase();
        let label = if *shifted == upper && *shifted != *unshifted {
            upper
        } else {
            unshifted.clone()
        };
        map.insert((*kc).to_string(), label);
    }
    for (alias, base) in SHIFT_ALIASES {
        if let Some((_, shifted)) = chars.get(base) {
            if !shifted.is_empty() {
                map.insert((*alias).to_string(), shifted.clone());
            }
        }
    }
    map
}

/// Glyphs for dead keys, which produce no character on their own.
fn dead_key_glyph(name: &str) -> Option<&'static str> {
    Some(match name {
        "circumflex" => "^",
        "diaeresis" => "¨",
        "grave" => "`",
        "acute" => "´",
        "tilde" => "~",
        "cedilla" => "¸",
        "caron" => "ˇ",
        "macron" => "¯",
        "breve" => "˘",
        "abovering" => "˚",
        "abovedot" => "˙",
        "doubleacute" => "˝",
        "ogonek" => "˛",
        _ => return None,
    })
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use xkbcommon::xkb;

    pub fn current_layout() -> Option<LayoutMap> {
        let (conn, _) = xcb::Connection::connect(None).ok()?;
        let mut major = 0;
        let mut minor = 0;
        let mut base_event = 0;
        let mut base_error = 0;
        xkb::x11::setup_xkb_extension(
            &conn,
            xkb::x11::MIN_MAJOR_XKB_VERSION,
            xkb::x11::MIN_MINOR_XKB_VERSION,
            xkb::x11::SetupXkbExtensionFlags::NoFlags,
            &mut major,
            &mut minor,
            &mut base_event,
            &mut base_error,
        )
        .then_some(())?;
        let device = xkb::x11::get_core_keyboard_device_id(&conn);
        if device < 0 {
            return None;
        }
        let ctx = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let keymap =
            xkb::x11::keymap_new_from_device(&ctx, &conn, device, xkb::KEYMAP_COMPILE_NO_FLAGS);
        let device_state = xkb::x11::state_new_from_device(&keymap, &conn, device);
        let group = device_state.serialize_layout(xkb::STATE_LAYOUT_EFFECTIVE);
        let name = keymap.layout_get_name(group).to_string();

        let shift = keymap.mod_get_index(xkb::MOD_NAME_SHIFT);
        if shift == xkb::MOD_INVALID {
            return None;
        }
        let mut plain = xkb::State::new(&keymap);
        plain.update_mask(0, 0, 0, 0, 0, group);
        let mut shifted = xkb::State::new(&keymap);
        shifted.update_mask(1 << shift, 0, 0, 0, 0, group);

        let key_char = |state: &xkb::State, keycode: u32| -> String {
            let key = (keycode + 8).into(); // evdev -> X keycode offset
            let utf8 = state.key_get_utf8(key);
            if !utf8.is_empty() && utf8.chars().all(|c| !c.is_control()) {
                return utf8;
            }
            let sym_name = xkb::keysym_get_name(state.key_get_one_sym(key));
            sym_name
                .strip_prefix("dead_")
                .and_then(dead_key_glyph)
                .unwrap_or("")
                .to_string()
        };

        let mut chars = HashMap::new();
        for (kc, evdev, _) in KEYS {
            chars.insert(*kc, (key_char(&plain, *evdev), key_char(&shifted, *evdev)));
        }
        Some(LayoutMap {
            name,
            map: build_map(chars),
        })
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use windows_sys::Win32::Globalization::LCIDToLocaleName;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetKeyboardLayout, MapVirtualKeyExW, ToUnicodeEx, MAPVK_VSC_TO_VK_EX, VK_SHIFT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    pub fn current_layout() -> Option<LayoutMap> {
        unsafe {
            let hwnd = GetForegroundWindow();
            let tid = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
            let hkl = GetKeyboardLayout(tid);

            let key_char = |sc: u16, with_shift: bool| -> String {
                let vk = MapVirtualKeyExW(sc as u32, MAPVK_VSC_TO_VK_EX, hkl);
                if vk == 0 {
                    return String::new();
                }
                let mut key_state = [0u8; 256];
                if with_shift {
                    key_state[VK_SHIFT as usize] = 0x80;
                }
                let mut buf = [0u16; 8];
                let n = ToUnicodeEx(vk, sc as u32, key_state.as_ptr(), buf.as_mut_ptr(), 8, 0, hkl);
                if n < 0 {
                    // Dead key: the glyph is in the buffer; call again to
                    // flush the keyboard's internal dead-key state.
                    let mut flush = [0u16; 8];
                    ToUnicodeEx(vk, sc as u32, key_state.as_ptr(), flush.as_mut_ptr(), 8, 0, hkl);
                    return String::from_utf16_lossy(&buf[..1]);
                }
                if n > 0 {
                    let s = String::from_utf16_lossy(&buf[..n as usize]);
                    if s.chars().all(|c| !c.is_control()) {
                        return s;
                    }
                }
                String::new()
            };

            let mut chars = HashMap::new();
            for (kc, _, sc) in KEYS {
                chars.insert(*kc, (key_char(*sc, false), key_char(*sc, true)));
            }

            let lcid = (hkl as usize & 0xFFFF) as u32;
            let mut name_buf = [0u16; 85];
            let len = LCIDToLocaleName(lcid, name_buf.as_mut_ptr(), 85, 0);
            let name = if len > 1 {
                String::from_utf16_lossy(&name_buf[..(len - 1) as usize])
            } else {
                format!("{lcid:04x}")
            };

            Some(LayoutMap {
                name,
                map: build_map(chars),
            })
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod platform {
    use super::*;

    pub fn current_layout() -> Option<LayoutMap> {
        None
    }
}
