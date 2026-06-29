use std::path::Path;

use tauri::path::BaseDirectory;
use tauri::{command, AppHandle, Manager};

use crate::service::excel::{self, GenerateResult};
use crate::utils::resolve_db_path;

/// Tauri command: read the uploaded invoices and fill the two saved templates.
///
/// `invoice_paths` – absolute paths to the source invoice .xlsx files.
/// `period`        – reporting period written on every row (e.g. "03/2026").
/// `output_dir`    – directory where `tracciati_pli.xlsx` / `tracciati_pat.xlsx` are written.
#[command]
pub async fn generate_tracciati(
    app_handle: AppHandle,
    invoice_paths: Vec<String>,
    period: String,
    output_dir: String,
) -> Result<GenerateResult, String> {
    let db_path = resolve_db_path(&app_handle)?;
    let pli_template = app_handle
        .path()
        .resolve("resources/tracciati_pli.xlsx", BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;
    let pat_template = app_handle
        .path()
        .resolve("resources/tracciati_pat.xlsx", BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;

    excel::generate_tracciati(
        invoice_paths,
        period,
        &pli_template,
        &pat_template,
        Path::new(&output_dir),
        &db_path,
    )
    .await
    .map_err(|e| e.to_string())
}
