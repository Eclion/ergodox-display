#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod hid;
mod settings;

use settings::{Settings, SettingsState, WindowMode};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, RunEvent, State, WebviewUrl,
    WebviewWindowBuilder, WindowEvent,
};

const MAIN_WINDOW: &str = "main";
const SETTINGS_WINDOW: &str = "settings";

#[tauri::command]
fn get_settings(state: State<SettingsState>) -> Settings {
    state.0.lock().unwrap().clone()
}

#[tauri::command]
fn get_kb_state(state: State<hid::KbState>) -> (hid::KbEvent, u8) {
    state.0.lock().unwrap().clone()
}

#[tauri::command]
fn set_mode(app: AppHandle, state: State<SettingsState>, mode: WindowMode) -> Result<(), String> {
    {
        let mut s = state.0.lock().unwrap();
        s.mode = mode;
        settings::save(&app, &s);
    }
    apply_mode(&app, mode);
    Ok(())
}

fn apply_mode(app: &AppHandle, mode: WindowMode) {
    if let Some(win) = app.get_webview_window(MAIN_WINDOW) {
        let _ = win.set_ignore_cursor_events(mode == WindowMode::Overlay);
    }
    let _ = app.emit("mode-changed", mode);
}

fn show_settings(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(SETTINGS_WINDOW) {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn setup(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let stored = settings::load(app.handle());
    let mode = stored.mode;
    let position = stored.position;
    app.manage(SettingsState(std::sync::Mutex::new(stored)));
    app.manage(hid::KbState::default());

    let main = WebviewWindowBuilder::new(app, MAIN_WINDOW, WebviewUrl::App("index.html".into()))
        .title("Ergodox Display")
        .inner_size(530.0, 582.0)
        .resizable(false)
        .maximizable(false)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .build()?;
    if let Some((x, y)) = position {
        let _ = main.set_position(PhysicalPosition::new(x, y));
    }
    let _ = main.set_ignore_cursor_events(mode == WindowMode::Overlay);

    WebviewWindowBuilder::new(
        app,
        SETTINGS_WINDOW,
        WebviewUrl::App("settings.html".into()),
    )
    .title("Ergodox Display — Settings")
    .inner_size(360.0, 280.0)
    .resizable(false)
    .visible(false)
    .build()?;

    let settings_item = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &settings_item,
            &PredefinedMenuItem::separator(app)?,
            &quit_item,
        ],
    )?;
    TrayIconBuilder::with_id("tray")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Ergodox Display")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "settings" => show_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    hid::spawn(app.handle().clone());
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_settings, set_mode, get_kb_state])
        .setup(|app| setup(app))
        .on_window_event(|window, event| match event {
            WindowEvent::Moved(pos) if window.label() == MAIN_WINDOW => {
                let state: State<SettingsState> = window.state();
                state.0.lock().unwrap().position = Some((pos.x, pos.y));
            }
            WindowEvent::CloseRequested { api, .. } if window.label() == SETTINGS_WINDOW => {
                api.prevent_close();
                let _ = window.hide();
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                let state: State<SettingsState> = app.state();
                let s = state.0.lock().unwrap().clone();
                settings::save(app, &s);
            }
        });
}
