#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod hid;
mod layoutstore;
mod oskeymap;
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
fn get_layout_map(state: State<oskeymap::LayoutMapState>) -> Option<oskeymap::LayoutMap> {
    state.0.lock().unwrap().clone()
}

#[tauri::command]
fn get_layout(state: State<layoutstore::LayoutState>) -> Option<serde_json::Value> {
    state.0.lock().unwrap().clone()
}

#[tauri::command]
async fn import_layout(app: AppHandle, source: String) -> Result<layoutstore::LayoutMeta, String> {
    // async so the blocking HTTP fetch runs off the main thread
    tauri::async_runtime::spawn_blocking(move || layoutstore::import(&app, &source))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn reset_layout(app: AppHandle) {
    layoutstore::reset(&app);
}

#[tauri::command]
fn get_app_info() -> serde_json::Value {
    serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "sha": env!("BUILD_GIT_SHA"),
        "builtAt": env!("BUILD_TIMESTAMP").parse::<u64>().unwrap_or(0),
    })
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
    app.manage(oskeymap::LayoutMapState(std::sync::Mutex::new(None)));
    app.manage(layoutstore::LayoutState(std::sync::Mutex::new(
        layoutstore::load_stored(app.handle()),
    )));

    let main = WebviewWindowBuilder::new(app, MAIN_WINDOW, WebviewUrl::App("index.html".into()))
        .title("Ergodox Display")
        .inner_size(530.0, 582.0)
        .resizable(false)
        .maximizable(false)
        .decorations(false)
        // On Windows, undecorated windows still get a DWM drop shadow with a
        // light border and rounded corners; disable it so the overlay is
        // pure content. No effect on Linux/X11.
        .shadow(false)
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
    .inner_size(380.0, 500.0)
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
    oskeymap::spawn(app.handle().clone());
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_mode,
            get_kb_state,
            get_layout_map,
            get_layout,
            import_layout,
            reset_layout,
            get_app_info
        ])
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
