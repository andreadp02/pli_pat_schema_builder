use std::path::Path;

use calamine::{open_workbook, Data, Reader, Xlsx, XlsxError};

use crate::service::excel::ExcelRow;
use crate::AppError;

/// A single value to write into an input column (1-based) of a templated data row.
#[derive(Debug, Clone)]
pub struct TemplateCell {
    pub column: u32,
    pub value: CellValue,
}

/// Text is written as-is; Date is written as a real Excel serial date (so ADM's importer sees a
/// date, not a string) with a dd/mm/yyyy display format.
#[derive(Debug, Clone)]
pub enum CellValue {
    Text(String),
    Date { year: i32, month: i32, day: i32 },
}

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

/// Fills a saved template in place: each data row is cloned from the template's prototype row at
/// `start_row` (so pre-filled constants, formulas and styling carry down), then the given input
/// cells overwrite that row's value columns. Formula cells keep their formula (row references
/// bumped); the output is written to `output_path`, leaving the template untouched.
pub async fn fill_template(
    template_path: &Path,
    output_path: &Path,
    sheet_name: &str,
    start_row: u32,
    rows: Vec<Vec<TemplateCell>>,
) -> Result<(), AppError> {
    let template_path = template_path.to_path_buf();
    let output_path = output_path.to_path_buf();
    let sheet_name = sheet_name.to_string();

    tauri::async_runtime::spawn_blocking(move || {
        let mut book = umya_spreadsheet::reader::xlsx::read(&template_path)
            .map_err(|e| AppError::Processing(format!("Cannot read template: {e}")))?;

        let sheet = book
            .get_sheet_by_name_mut(&sheet_name)
            .ok_or_else(|| AppError::Processing(format!("Template missing sheet '{sheet_name}'")))?;

        let (max_col, _) = sheet.get_highest_column_and_row();

        // Snapshot the prototype row up front (owned) so the write loop can borrow the sheet mutably.
        let mut prototype: Vec<(u32, umya_spreadsheet::Style, ProtoCell)> = Vec::new();
        for col in 1..=max_col {
            let (style, proto) = match sheet.get_cell((col, start_row)) {
                Some(cell) => {
                    let proto = if cell.get_formula().is_empty() {
                        ProtoCell::Value(cell.get_value().to_string())
                    } else {
                        ProtoCell::Formula(cell.get_formula().to_string())
                    };
                    (cell.get_style().clone(), proto)
                }
                None => (umya_spreadsheet::Style::default(), ProtoCell::Value(String::new())),
            };
            prototype.push((col, style, proto));
        }

        for (offset, row_cells) in rows.iter().enumerate() {
            let target_row = start_row + offset as u32;

            for (col, style, proto) in &prototype {
                let cell = sheet.get_cell_mut((*col, target_row));
                cell.set_style(style.clone());
                match proto {
                    ProtoCell::Formula(formula) => {
                        cell.set_formula(bump_formula_rows(formula, start_row, target_row));
                    }
                    ProtoCell::Value(value) => {
                        cell.set_value(value.clone());
                    }
                }
            }

            for input in row_cells {
                let cell = sheet.get_cell_mut((input.column, target_row));
                match &input.value {
                    CellValue::Text(value) => {
                        cell.set_value(value.clone());
                    }
                    CellValue::Date { year, month, day } => {
                        let serial =
                            umya_spreadsheet::helper::date::convert_date(*year, *month, *day, 0, 0, 0);
                        cell.set_value_number(serial);
                        cell.get_style_mut()
                            .get_number_format_mut()
                            .set_format_code("dd/mm/yyyy");
                    }
                }
            }
        }

        umya_spreadsheet::writer::xlsx::write(&book, &output_path)
            .map_err(|e| AppError::Processing(format!("Cannot write output: {e}")))?;

        Ok(())
    })
    .await
    .map_err(|e| AppError::Processing(format!("Template fill task failed: {e}")))?
}

enum ProtoCell {
    Value(String),
    Formula(String),
}

/// Bumps the row component of A1-style cell references equal to `from_row` to `to_row`, leaving
/// column letters, absolute markers and plain numbers untouched.
// ponytail: handles relative same-row refs (the only kind these templates use); cross-row or
// range refs are left as-is — revisit if a template needs them.
fn bump_formula_rows(formula: &str, from_row: u32, to_row: u32) -> String {
    if from_row == to_row {
        return formula.to_string();
    }
    let from = from_row.to_string();
    let to = to_row.to_string();
    let bytes = formula.as_bytes();
    let mut out = String::with_capacity(formula.len());
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if !ch.is_ascii_alphabetic() {
            out.push(ch);
            i += 1;
            continue;
        }
        let letters_start = i;
        while i < bytes.len() && (bytes[i] as char).is_ascii_alphabetic() {
            i += 1;
        }
        out.push_str(&formula[letters_start..i]);
        if i < bytes.len() && bytes[i] == b'$' {
            out.push('$');
            i += 1;
        }
        let digits_start = i;
        while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
            i += 1;
        }
        let digits = &formula[digits_start..i];
        if !digits.is_empty() && digits == from {
            out.push_str(&to);
        } else {
            out.push_str(digits);
        }
    }
    out
}

