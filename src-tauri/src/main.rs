#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod hid;

use tauri::{WebviewUrl, WebviewWindowBuilder};

const MAIN_WINDOW: &str = "main";

fn setup(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    WebviewWindowBuilder::new(app, MAIN_WINDOW, WebviewUrl::App("index.html".into()))
        .title("Ergodox Display")
        .inner_size(530.0, 582.0)
        .resizable(false)
        .maximizable(false)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .build()?;
    hid::spawn(app.handle().clone());
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| setup(app))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
