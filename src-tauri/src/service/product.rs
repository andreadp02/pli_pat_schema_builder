use std::path::Path;

use crate::repository::excel as excel_repository;
use crate::repository::product::{self, NewProduct};
use crate::service::excel::ExcelRow;
use crate::utils::parse_i64;
use crate::AppError;

const CODE_COLUMN_INDEX: usize = 5; // F
const DESCRIPTION_COLUMN_INDEX: usize = 6; // G
const UNITS_COLUMN_INDEX: usize = 8; // I
const PLI_COLUMN_INDEX: usize = 10; // K
const DATA_START_ROW_INDEX: usize = 1;
const PLI_TRUE_VALUE: i64 = 5;

#[derive(Debug)]
struct ParsedProductRow {
    code: String,
    description: String,
    units: u32,
    pli: bool,
}

pub async fn upload_products_excel(file_path: &Path, db_path: &Path) -> Result<String, AppError> {
    if !file_path.is_file() {
        return Err(AppError::Processing("Selected file does not exist".to_string()));
    }

    let ext = file_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();

    if !ext.eq_ignore_ascii_case("xlsx") {
        return Err(AppError::Processing(
            "Only .xlsx files are supported".to_string(),
        ));
    }

    let rows = excel_repository::read_excel(file_path).await?;

    if rows.len() <= DATA_START_ROW_INDEX {
        return Err(AppError::Processing(
            "Excel file must contain at least two rows".to_string(),
        ));
    }

    let products = parse_products_rows(&rows)?;

    for product in &products {
        println!(
            "Products upload extracted -> code: {}, description: {}, units: {}, pli: {}",
            product.code, product.description, product.units, product.pli
        );

        product::create_product(
            db_path.to_path_buf(),
            NewProduct {
                code: product.code.clone(),
                description: product.description.clone(),
                units: product.units,
                pli: product.pli,
            },
        )
        .await?;
    }

    Ok(format!("Imported {} products successfully", products.len()))
}

fn parse_products_rows(rows: &[ExcelRow]) -> Result<Vec<ParsedProductRow>, AppError> {
    rows.iter()
        .enumerate()
        .skip(DATA_START_ROW_INDEX)
        .map(|(row_index, row)| parse_product_row(row_index, row))
        .collect()
}

fn parse_product_row(row_index: usize, row: &ExcelRow) -> Result<ParsedProductRow, AppError> {
    let row_number = row_index + 1;

    let code = get_required_cell(row, CODE_COLUMN_INDEX, row_number, "code")?;
    let description = get_required_cell(row, DESCRIPTION_COLUMN_INDEX, row_number, "description")?;
    let units = parse_non_negative_u32(
        get_required_cell(row, UNITS_COLUMN_INDEX, row_number, "units")?,
        row_number,
        "units",
    )?;
    let pli_value = parse_i64(
        get_required_cell(row, PLI_COLUMN_INDEX, row_number, "pli")?,
        row_number,
        "pli",
    )?;

    Ok(ParsedProductRow {
        code: code.to_string(),
        description: description.to_string(),
        units,
        pli: pli_value == PLI_TRUE_VALUE,
    })
}

fn get_required_cell<'a>(
    row: &'a ExcelRow,
    column_index: usize,
    row_number: usize,
    field_name: &str,
) -> Result<&'a str, AppError> {
    let value = row
        .cells
        .get(column_index)
        .map(|cell| cell.trim())
        .filter(|cell| !cell.is_empty())
        .ok_or_else(|| {
            AppError::Processing(format!(
                "Missing {field_name} at row {row_number}, column {}",
                column_index + 1
            ))
        })?;

    Ok(value)
}

fn parse_non_negative_u32(value: &str, row_number: usize, field_name: &str) -> Result<u32, AppError> {
    let parsed = parse_i64(value, row_number, field_name)?;
    if parsed < 0 {
        return Err(AppError::Processing(format!(
            "Invalid {field_name} at row {row_number}: value cannot be negative"
        )));
    }

    u32::try_from(parsed).map_err(|_| {
        AppError::Processing(format!(
            "Invalid {field_name} at row {row_number}: value is out of range"
        ))
    })
}
