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
/// Transformations are intentionally left as no-ops; extend `transform_*` as needed.
pub fn process_excel(input_path: &Path, output_dir: &Path) -> Result<ProcessResult, AppError> {
    let rows = repository::read_excel(input_path)?;

    let output1_rows = transform_output1(&rows);
    let output2_rows = transform_output2(&rows);

    let output1_path = output_dir.join("output1.xlsx");
    let output2_path = output_dir.join("output2.xlsx");

    repository::write_excel(&output1_path, "Output1", &output1_rows)?;
    repository::write_excel(&output2_path, "Output2", &output2_rows)?;

    Ok(ProcessResult {
        output1: path_to_string(&output1_path),
        output2: path_to_string(&output2_path),
    })
}

// ---------------------------------------------------------------------------
// Transformation helpers – extend these to add business logic.
// ---------------------------------------------------------------------------

fn transform_output1(rows: &[ExcelRow]) -> Vec<ExcelRow> {
    // Placeholder: return data as-is for now.
    rows.to_vec()
}

fn transform_output2(rows: &[ExcelRow]) -> Vec<ExcelRow> {
    // Placeholder: return data as-is for now.
    rows.to_vec()
}

fn path_to_string(path: &PathBuf) -> String {
    path.to_string_lossy().into_owned()
}
