use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::repository::excel as excel_repository;
use crate::AppError;

/// Represents a single row from an Excel sheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelRow {
    pub cells: Vec<String>,
}

/// Result returned to the frontend after processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResult {
    pub output1: String,
    pub output2: String,
}

/// Reads the input Excel file, applies transformations, and writes two output files.
///
/// Current behavior preserves workbook structure and formatting by cloning the
/// original file to each output.
pub async fn process_excel(
    input_path: &Path,
    output_dir: &Path,
) -> Result<ProcessResult, AppError> {
    let input_stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let input_ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("xlsx");

    let output1_path = output_dir.join(format!("{}_output1.{}", input_stem, input_ext));
    let output2_path = output_dir.join(format!("{}_output2.{}", input_stem, input_ext));

    let input1 = input_path.to_path_buf();
    let input2 = input_path.to_path_buf();
    let output1 = output1_path.clone();
    let output2 = output2_path.clone();

    let copy1 = tauri::async_runtime::spawn(async move {
        excel_repository::copy_excel_file(&input1, &output1).await
    });
    let copy2 = tauri::async_runtime::spawn(async move {
        excel_repository::copy_excel_file(&input2, &output2).await
    });

    copy1
        .await
        .map_err(|e| AppError::Processing(format!("Copy task 1 failed: {e}")))??;
    copy2
        .await
        .map_err(|e| AppError::Processing(format!("Copy task 2 failed: {e}")))??;

    Ok(ProcessResult {
        output1: path_to_string(&output1_path),
        output2: path_to_string(&output2_path),
    })
}

fn path_to_string(path: &PathBuf) -> String {
    path.to_string_lossy().into_owned()
}
