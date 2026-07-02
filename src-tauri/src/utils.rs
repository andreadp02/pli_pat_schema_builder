use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use tauri::Manager;

use crate::service::excel::ExcelRow;
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

/// Maps each normalized header cell to its column index, so uploads locate columns by name
/// instead of a hardcoded position (exports vary in column order/count between sources).
pub fn build_header_map(header_row: &ExcelRow) -> HashMap<String, usize> {
	let mut headers = HashMap::new();
	for (index, value) in header_row.cells.iter().enumerate() {
		let key = normalize_header(value);
		if !key.is_empty() {
			headers.insert(key, index);
		}
	}
	headers
}

/// Lowercases and collapses runs of non-alphanumeric characters into a single '_', so header
/// matching tolerates spacing/casing/punctuation differences between exports (e.g. "Info 3" -> "info_3").
fn normalize_header(value: &str) -> String {
	value
		.trim()
		.to_lowercase()
		.chars()
		.map(|c| if c.is_alphanumeric() { c } else { '_' })
		.collect::<String>()
		.split('_')
		.filter(|part| !part.is_empty())
		.collect::<Vec<_>>()
		.join("_")
}

pub fn find_required_header(
	headers: &HashMap<String, usize>,
	candidates: &[&str],
) -> Result<usize, AppError> {
	find_optional_header(headers, candidates).ok_or_else(|| {
		AppError::Processing(format!(
			"Missing required header. Accepted names: {}",
			candidates.join(", ")
		))
	})
}

pub fn find_optional_header(headers: &HashMap<String, usize>, candidates: &[&str]) -> Option<usize> {
	candidates.iter().find_map(|name| headers.get(*name).copied())
}
