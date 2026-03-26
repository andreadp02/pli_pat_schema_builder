use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::repository;
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
pub fn process_excel(input_path: &Path, output_dir: &Path) -> Result<ProcessResult, AppError> {
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

    repository::copy_excel_file(input_path, &output1_path)?;
    repository::copy_excel_file(input_path, &output2_path)?;

    Ok(ProcessResult {
        output1: path_to_string(&output1_path),
        output2: path_to_string(&output2_path),
    })
}

fn path_to_string(path: &PathBuf) -> String {
    path.to_string_lossy().into_owned()
}
