mod controller;
mod repository;
mod service;
mod utils;

use std::fs;

#[cfg(target_os = "windows")]
use std::fs::OpenOptions;
#[cfg(target_os = "windows")]
use std::process::Command;

use rusqlite::Connection;
use tauri::Manager;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Processing error: {0}")]
    Processing(String),
}

#[tauri::command]
fn window_minimize(window: tauri::Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
fn window_toggle_maximize(window: tauri::Window) -> Result<(), String> {
    let is_maximized = window.is_maximized().map_err(|e| e.to_string())?;
    if is_maximized {
        window.unmaximize().map_err(|e| e.to_string())
    } else {
        window.maximize().map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn window_close(window: tauri::Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[tauri::command]
fn window_start_dragging(window: tauri::Window) -> Result<(), String> {
    window.start_dragging().map_err(|e| e.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            ensure_product_table_on_startup(app.handle())?;

            #[cfg(target_os = "windows")]
            if let Err(err) = hide_database_file_on_windows(app.handle()) {
                log::warn!("Failed to hide database file: {err}");
            }

            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            window_minimize,
            window_toggle_maximize,
            window_close,
            window_start_dragging,
            controller::excel::process_excel_file,
            controller::product::create_product,
            controller::product::get_products,
            controller::product::get_product_by_id,
            controller::product::get_product_by_code,
            controller::product::update_product,
            controller::product::delete_product,
            controller::product::upload_products_excel
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn ensure_product_table_on_startup(app: &tauri::AppHandle) -> Result<(), String> {
    let db_path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("products.db");

    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS product (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            code TEXT NOT NULL CHECK(length(code) > 0) UNIQUE,
            description TEXT,
            units INTEGER NOT NULL,
            pli INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn hide_database_file_on_windows(app: &tauri::AppHandle) -> Result<(), String> {
    let db_path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("products.db");

    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&db_path)
        .map_err(|e| e.to_string())?;

    let status = Command::new("attrib")
        .arg("+H")
        .arg(&db_path)
        .status()
        .map_err(|e| e.to_string())?;

    if !status.success() {
        return Err("attrib command failed".to_string());
    }

    Ok(())
}
