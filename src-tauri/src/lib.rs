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
use tauri_plugin_sql::{Migration, MigrationKind};
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
    let sql_migrations = vec![Migration {
        version: 1,
        description: "create_product_table",
        sql: "
            CREATE TABLE IF NOT EXISTS product (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                code TEXT NOT NULL UNIQUE,
                description TEXT,
                units INTEGER NOT NULL,
                pli INTEGER NOT NULL DEFAULT 0
            );
        ",
        kind: MigrationKind::Up,
    }, Migration {
        version: 2,
        description: "create_customer_and_geography_tables",
        sql: "
            CREATE TABLE IF NOT EXISTS province (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                code TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL UNIQUE
            );

            CREATE TABLE IF NOT EXISTS municipality (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                code TEXT UNIQUE,
                name TEXT NOT NULL,
                province_id INTEGER NOT NULL,
                FOREIGN KEY (province_id) REFERENCES province(id),
                UNIQUE(name, province_id)
            );

            CREATE TABLE IF NOT EXISTS customer (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tax_code INTEGER NOT NULL UNIQUE,
                ordinal_number INTEGER NOT NULL,
                typology TEXT NOT NULL CHECK (typology IN ('ESERCIZIO DI VICINATO','RIVENDITA','FARMACIA','PARAFARMACIA')),
                vat_number TEXT UNIQUE,
                address TEXT NOT NULL,
                municipality_id INTEGER NOT NULL,
                FOREIGN KEY (municipality_id) REFERENCES municipality(id)
            );

            CREATE INDEX IF NOT EXISTS idx_municipality_province_id ON municipality(province_id);
            CREATE INDEX IF NOT EXISTS idx_customer_municipality_id ON customer(municipality_id);
        ",
        kind: MigrationKind::Up,
    }, Migration {
        version: 3,
        description: "simplify_municipality_and_province_schema",
        sql: "
            PRAGMA foreign_keys = OFF;

            CREATE TABLE IF NOT EXISTS municipality_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                province_name TEXT NOT NULL,
                UNIQUE(name, province_name)
            );

            INSERT OR IGNORE INTO municipality_new (name, province_name)
            SELECT m.name, COALESCE(p.name, '')
            FROM municipality m
            LEFT JOIN province p ON p.id = m.province_id;

            CREATE TABLE IF NOT EXISTS customer_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tax_code INTEGER NOT NULL UNIQUE,
                ordinal_number INTEGER NOT NULL,
                typology TEXT NOT NULL CHECK (typology IN ('ESERCIZIO DI VICINATO','RIVENDITA','FARMACIA','PARAFARMACIA')),
                vat_number TEXT UNIQUE,
                address TEXT NOT NULL,
                municipality_id INTEGER NOT NULL,
                FOREIGN KEY (municipality_id) REFERENCES municipality_new(id)
            );

            INSERT OR IGNORE INTO customer_new (id, tax_code, ordinal_number, typology, vat_number, address, municipality_id)
            SELECT c.id, c.tax_code, c.ordinal_number, c.typology, c.vat_number, c.address, mn.id
            FROM customer c
            JOIN municipality m ON m.id = c.municipality_id
            LEFT JOIN province p ON p.id = m.province_id
            JOIN municipality_new mn
                ON mn.name = m.name
               AND mn.province_name = COALESCE(p.name, '');

            DROP TABLE IF EXISTS customer;
            DROP TABLE IF EXISTS municipality;
            DROP TABLE IF EXISTS province;

            ALTER TABLE municipality_new RENAME TO municipality;
            ALTER TABLE customer_new RENAME TO customer;

            CREATE INDEX IF NOT EXISTS idx_customer_municipality_id ON customer(municipality_id);
            CREATE INDEX IF NOT EXISTS idx_municipality_name_province ON municipality(name, province_name);

            PRAGMA foreign_keys = ON;
        ",
        kind: MigrationKind::Up,
    }];

    tauri::Builder::default()
        .plugin(
            tauri_plugin_sql::Builder::new()
                .add_migrations("sqlite:products.db", sql_migrations)
                .build(),
        )
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
            controller::excel::process_excel_file,
            controller::product::create_product,
            controller::product::get_products,
            controller::product::get_product_by_id,
            controller::product::get_product_by_code,
            controller::product::update_product,
            controller::product::delete_product,
            controller::product::upload_products_excel,
            controller::customer::create_customer,
            controller::customer::get_customers,
            controller::customer::get_customer_by_id,
            controller::customer::update_customer,
            controller::customer::delete_customer,
            controller::customer::upload_customers_excel
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
            code TEXT NOT NULL UNIQUE,
            description TEXT,
            units INTEGER NOT NULL,
            pli INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

fn ensure_customer_tables_on_startup(app: &tauri::AppHandle) -> Result<(), String> {
    let db_path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("products.db");

    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

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
            vat_number TEXT UNIQUE,
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
