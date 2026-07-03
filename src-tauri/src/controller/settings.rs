use tauri::{command, AppHandle};

use crate::service::settings::{self, AccisaCoefficients};
use crate::utils::resolve_db_path;

#[command]
pub async fn get_accisa_coefficients(app_handle: AppHandle) -> Result<AccisaCoefficients, String> {
    let db_path = resolve_db_path(&app_handle)?;
    settings::get_accisa_coefficients(&db_path)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn save_accisa_coefficients(
    app_handle: AppHandle,
    coefficients: AccisaCoefficients,
) -> Result<(), String> {
    let db_path = resolve_db_path(&app_handle)?;
    settings::save_accisa_coefficients(&db_path, coefficients)
        .await
        .map_err(|e| e.to_string())
}
