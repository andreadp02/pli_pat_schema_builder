use std::path::Path;
use std::{fs, io};

use calamine::{open_workbook, Data, Reader, Xlsx, XlsxError};
use rust_xlsxwriter::Workbook;

use crate::service::ExcelRow;
use crate::AppError;

/// Reads all rows from the first sheet of an Excel (.xlsx) file.
pub fn read_excel(path: &Path) -> Result<Vec<ExcelRow>, AppError> {
    let mut workbook: Xlsx<_> =
        open_workbook(path).map_err(|e: XlsxError| AppError::Io(e.to_string()))?;

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
}

/// Writes a list of rows to an Excel (.xlsx) file at the given path.
pub fn write_excel(path: &Path, sheet_name: &str, rows: &[ExcelRow]) -> Result<(), AppError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet
        .set_name(sheet_name)
        .map_err(|e| AppError::Processing(e.to_string()))?;

    for (row_idx, row) in rows.iter().enumerate() {
        for (col_idx, cell) in row.cells.iter().enumerate() {
            worksheet
                .write_string(row_idx as u32, col_idx as u16, cell)
                .map_err(|e| AppError::Processing(e.to_string()))?;
        }
    }

    workbook
        .save(path)
        .map_err(|e| AppError::Processing(e.to_string()))?;

    Ok(())
}

/// Copies an Excel file preserving all sheets, formulas, and formatting.
pub fn copy_excel_file(input_path: &Path, output_path: &Path) -> Result<(), AppError> {
    fs::copy(input_path, output_path)
        .map(|_| ())
        .map_err(|e: io::Error| AppError::Io(e.to_string()))
}
