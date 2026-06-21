// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::check_prerequisites,
            commands::extract,
            commands::extract_text,
            commands::analyze_recipe,
            commands::import_cookies,
            commands::check_cookies,
            commands::list_recipes,
            commands::get_recipe,
            commands::delete_recipe,
            commands::save_recipe,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
