use std::path::Path;
use std::{fs, io};

use calamine::{open_workbook, Data, Reader, Xlsx, XlsxError};
use rust_xlsxwriter::Workbook;

use crate::service::excel::ExcelRow;
use crate::AppError;

/// Reads all rows from the first sheet of an Excel (.xlsx) file.
pub async fn read_excel(path: &Path) -> Result<Vec<ExcelRow>, AppError> {
    let path = path.to_path_buf();

    tauri::async_runtime::spawn_blocking(move || {
        let mut workbook: Xlsx<_> =
            open_workbook(&path).map_err(|e: XlsxError| AppError::Io(e.to_string()))?;

        let sheet_names: Vec<String> = workbook.sheet_names().to_owned();
        let sheet_name = sheet_names
            .first()
            .ok_or_else(|| AppError::Processing("Excel file has no sheets".into()))?
            .clone();

        let range = workbook
            .worksheet_range(&sheet_name)
            .map_err(|e: XlsxError| AppError::Processing(e.to_string()))?;

        let rows: Vec<ExcelRow> = range
            .rows()
            .map(|row: &[Data]| ExcelRow {
                cells: row.iter().map(|cell: &Data| cell.to_string()).collect(),
            })
            .collect();

        Ok(rows)
    })
    .await
    .map_err(|e| AppError::Processing(format!("Background task failed: {e}")))?
}

/// Writes a list of rows to an Excel (.xlsx) file at the given path.
pub async fn write_excel(path: &Path, sheet_name: &str, rows: &[ExcelRow]) -> Result<(), AppError> {
    let path = path.to_path_buf();
    let sheet_name = sheet_name.to_string();
    let rows = rows.to_vec();

    tauri::async_runtime::spawn_blocking(move || {
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet
            .set_name(&sheet_name)
            .map_err(|e| AppError::Processing(e.to_string()))?;

        for (row_idx, row) in rows.iter().enumerate() {
            for (col_idx, cell) in row.cells.iter().enumerate() {
                worksheet
                    .write_string(row_idx as u32, col_idx as u16, cell)
                    .map_err(|e| AppError::Processing(e.to_string()))?;
            }
        }

        workbook
            .save(&path)
            .map_err(|e| AppError::Processing(e.to_string()))?;

        Ok(())
    })
    .await
    .map_err(|e| AppError::Processing(format!("Background task failed: {e}")))?
}

/// Copies an Excel file preserving all sheets, formulas, and formatting.
pub async fn copy_excel_file(input_path: &Path, output_path: &Path) -> Result<(), AppError> {
    let input = input_path.to_path_buf();
    let output = output_path.to_path_buf();

    tauri::async_runtime::spawn_blocking(move || {
        fs::copy(input, output)
            .map(|_| ())
            .map_err(|e: io::Error| AppError::Io(e.to_string()))
    })
    .await
    .map_err(|e| AppError::Processing(format!("Background task failed: {e}")))?
}
