use std::fs;
use std::path::PathBuf;

use tauri::Manager;

use crate::AppError;

pub const DB_FILE_NAME: &str = "pli_pat.db";
pub const TEMPLATE_PLI_FILE_NAME: &str = "tracciati_pli.xlsx";
pub const TEMPLATE_PAT_FILE_NAME: &str = "tracciati_pat.xlsx";

pub fn resolve_db_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
	resolve_app_data_path(app_handle, DB_FILE_NAME)
}

/// Path to a user-uploaded template stored in the app data dir (next to the db).
pub fn resolve_template_path(
	app_handle: &tauri::AppHandle,
	file_name: &str,
) -> Result<PathBuf, String> {
	resolve_app_data_path(app_handle, file_name)
}

fn resolve_app_data_path(app_handle: &tauri::AppHandle, file_name: &str) -> Result<PathBuf, String> {
	let mut dir = app_handle
		.path()
		.app_data_dir()
		.map_err(|e| format!("Failed to resolve app data dir: {e}"))?;

	fs::create_dir_all(&dir).map_err(|e| format!("Failed to create app data dir: {e}"))?;
	dir.push(file_name);
	Ok(dir)
}

pub fn parse_i64(value: &str, row_number: usize, field_name: &str) -> Result<i64, AppError> {
	if let Ok(parsed) = value.parse::<i64>() {
		return Ok(parsed);
	}

	if let Ok(parsed) = value.parse::<f64>() {
		if parsed.fract() == 0.0 {
			return Ok(parsed as i64);
		}
	}

	Err(AppError::Processing(format!(
		"Invalid {field_name} at row {row_number}: '{value}' is not an integer"
	)))
}
