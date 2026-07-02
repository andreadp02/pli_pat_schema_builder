mod controller;
mod repository;
mod service;
mod utils;

#[cfg(target_os = "windows")]
use std::fs::OpenOptions;
#[cfg(target_os = "windows")]
use std::process::Command;

use rusqlite::Connection;
use thiserror::Error;

use crate::utils::{resolve_db_path};

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

/// Open a file with the OS default handler (e.g. the generated tracciati in Excel).
// ponytail: std::process instead of tauri-plugin-opener — saves a dependency for 3 lines.
#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("cmd")
        .args(["/C", "start", "", &path])
        .spawn();
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(&path).spawn();
    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open").arg(&path).spawn();
    result.map(|_| ()).map_err(|e| e.to_string())
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
            ensure_customer_tables_on_startup(app.handle())?;

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
            open_path,
            controller::tracciati::generate_tracciati,
            controller::template::save_template,
            controller::template::get_templates_status,
            controller::product::create_product,
            controller::product::get_products,
            controller::product::get_product_by_id,
            controller::product::get_product_by_code,
            controller::product::update_product,
            controller::product::delete_product,
            controller::product::upload_products_excel,
            controller::product::upload_skeleton_excel,
            controller::customer::create_customer,
            controller::customer::get_customers,
            controller::customer::get_customer_by_tax_code,
            controller::customer::get_customer_by_vat_number,
            controller::customer::get_customer_by_id,
            controller::customer::update_customer,
            controller::customer::delete_customer,
            controller::customer::upload_customers_excel,
            controller::customer::validate_customers_excel,
            controller::customer::confirm_customers_excel_upload
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn ensure_product_table_on_startup(app: &tauri::AppHandle) -> Result<(), String> {
    let db_path = resolve_db_path(app)?;

    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS product (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            product_type TEXT NOT NULL CHECK(product_type IN ('pli','pat')),
            code TEXT NOT NULL CHECK(length(code) > 0) UNIQUE,
            description TEXT,
            units INTEGER NOT NULL,
            capacity INTEGER,
            nicotine INTEGER,
            packages INTEGER,
            adm_code TEXT,
            tabella INTEGER,
            CHECK (
                (product_type = 'pli' AND packages IS NULL)
             OR (product_type = 'pat' AND capacity IS NULL AND nicotine IS NULL)
            )
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    // Idempotent migration for DBs created before adm_code/tabella existed.
    add_column_if_missing(&conn, "product", "adm_code", "TEXT")?;
    add_column_if_missing(&conn, "product", "tabella", "INTEGER")?;

    Ok(())
}

/// Adds `ALTER TABLE <table> ADD COLUMN <column> <decl>` only when the column is absent.
fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    decl: &str,
) -> Result<(), String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info(?1) WHERE name = ?2",
            rusqlite::params![table, column],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if count == 0 {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {decl}"),
            [],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn ensure_customer_tables_on_startup(app: &tauri::AppHandle) -> Result<(), String> {
    let db_path = resolve_db_path(app)?;

    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("PRAGMA foreign_keys = ON", [])
        .map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS municipality (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            province_name TEXT NOT NULL,
            UNIQUE(name, province_name)
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS customer (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tax_code INTEGER NOT NULL UNIQUE,
            ordinal_number INTEGER NOT NULL,
            typology TEXT NOT NULL CHECK (typology IN ('ESERCIZIO DI VICINATO','RIVENDITA','FARMACIA','PARAFARMACIA')),
            vat_number TEXT,
            address TEXT NOT NULL,
            municipality_id INTEGER NOT NULL,
            FOREIGN KEY (municipality_id) REFERENCES municipality(id)
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_customer_municipality_id ON customer(municipality_id)",
        [],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_municipality_name_province ON municipality(name, province_name)",
        [],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn hide_database_file_on_windows(app: &tauri::AppHandle) -> Result<(), String> {
    let db_path = resolve_db_path(app)?;

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
