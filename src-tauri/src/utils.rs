use std::fs;
use std::path::PathBuf;

use tauri::Manager;

use crate::AppError;

pub const DB_FILE_NAME: &str = "pli_pat.db";
pub const SQLITE_DB_URL: &str = "sqlite:pli_pat.db";

pub fn resolve_db_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
	let mut db_dir = app_handle
		.path()
		.app_data_dir()
		.map_err(|e| format!("Failed to resolve app data dir: {e}"))?;

	fs::create_dir_all(&db_dir).map_err(|e| format!("Failed to create app data dir: {e}"))?;
	db_dir.push(DB_FILE_NAME);
	Ok(db_dir)
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
