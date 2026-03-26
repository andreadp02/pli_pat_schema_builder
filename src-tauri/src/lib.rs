mod controller;
mod repository;
mod service;

use tauri_plugin_sql::{Migration, MigrationKind};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Processing error: {0}")]
    Processing(String),
}

pub fn run() {
    let sql_migrations = vec![Migration {
        version: 1,
        description: "create_product_table",
        sql: "
            CREATE TABLE IF NOT EXISTS product (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                code TEXT NOT NULL,
                description TEXT NOT NULL,
                units INTEGER NOT NULL,
                pli INTEGER NOT NULL DEFAULT 0
            );
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
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![controller::process_excel_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
