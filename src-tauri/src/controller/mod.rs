use std::path::Path;

use tauri::command;

use crate::service::{self, ProcessResult};

/// Tauri command: process an uploaded Excel file and produce two output files.
///
/// `input_path`  – absolute path to the source .xlsx file chosen by the user.
/// `output_dir`  – directory where `output1.xlsx` and `output2.xlsx` will be written.
#[command]
pub fn process_excel_file(
    input_path: String,
    output_dir: String,
) -> Result<ProcessResult, String> {
    let input = Path::new(&input_path);
    let output = Path::new(&output_dir);

    service::process_excel(input, output).map_err(|e| e.to_string())
}
