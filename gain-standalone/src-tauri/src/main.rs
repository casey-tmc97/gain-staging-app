#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod dto;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::file::import_file,
            commands::analyze::analyze,
            commands::version::get_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
