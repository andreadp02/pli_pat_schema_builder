use std::collections::HashMap;
use std::path::Path;

use crate::repository::customer::{self, NewCustomer};
use crate::repository::excel as excel_repository;
use crate::service::excel::ExcelRow;
use crate::utils::parse_i64;
use crate::AppError;

const HEADER_ROW_INDEX: usize = 0;
const DATA_START_ROW_INDEX: usize = 1;

pub async fn upload_customers_excel(file_path: &Path, db_path: &Path) -> Result<String, AppError> {
    if !file_path.is_file() {
        return Err(AppError::Processing("Selected file does not exist".to_string()));
    }

    let ext = file_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();

    if !ext.eq_ignore_ascii_case("xlsx") {
        return Err(AppError::Processing("Only .xlsx files are supported".to_string()));
    }

    let rows = excel_repository::read_excel(file_path).await?;
    if rows.len() <= DATA_START_ROW_INDEX {
        return Err(AppError::Processing(
            "Excel file must contain a header row and at least one data row".to_string(),
        ));
    }

    let customers = parse_customer_rows(&rows)?;
    let inserted = customer::create_customers_bulk(db_path.to_path_buf(), customers).await?;
    Ok(format!("Imported {inserted} customers successfully"))
}

fn parse_customer_rows(rows: &[ExcelRow]) -> Result<Vec<NewCustomer>, AppError> {
    let header_row = rows
        .get(HEADER_ROW_INDEX)
        .ok_or_else(|| AppError::Processing("Missing header row".to_string()))?;

    let headers = build_header_map(header_row);
    let tax_code_idx = find_required_header(
        &headers,
        &[
            "tax_code",
            "taxcode",
            "numero_esercizio_vicinato_cmnr_rivendita",
        ],
    )?;
    let ordinal_number_idx = find_required_header(
        &headers,
        &[
            "numero_ordinale",
            "numeroordinale",
            "num_ordinale_punto_vendita",
        ],
    )?;
    let typology_idx = find_required_header(
        &headers,
        &["typology", "tipologia", "tipologia_punto_vendita"],
    )?;
    let vat_idx = find_optional_header(
        &headers,
        &[
            "vat_number",
            "vatnumber",
            "partita_iva",
            "partitaiva",
            "cf_piva_punto_vendita",
        ],
    );
    let address_idx = find_required_header(
        &headers,
        &["address", "indirizzo", "indirizzo_punto_vendita"],
    )?;
    let province_name_idx = find_required_header(
        &headers,
        &[
            "provincia",
            "nome_provincia",
        ],
    )?;
    let municipality_name_idx = find_required_header(
        &headers,
        &[
            "comune",
            "nome_comune",
            "comune_punto_vendita",
        ],
    )?;

    rows.iter()
        .enumerate()
        .skip(DATA_START_ROW_INDEX)
        .map(|(row_index, row)| {
            let row_number = row_index + 1;
            let tax_code = parse_i64(
                get_required_by_index(row, tax_code_idx, row_number, "tax_code")?,
                row_number,
                "tax_code",
            )?;
            let ordinal_number = parse_i64(
                get_required_by_index(row, ordinal_number_idx, row_number, "ordinal_number")?,
                row_number,
                "ordinal_number",
            )?;

            Ok(NewCustomer {
                tax_code,
                ordinal_number,
                typology: get_required_by_index(row, typology_idx, row_number, "typology")?.to_string(),
                vat_number: vat_idx.and_then(|idx| get_optional_by_index(row, idx)),
                address: get_required_by_index(row, address_idx, row_number, "address")?.to_string(),
                municipality_name: get_required_by_index(row, municipality_name_idx, row_number, "municipality_name")?
                    .to_string(),
                province_name: get_required_by_index(row, province_name_idx, row_number, "province_name")?
                    .to_string(),
            })
        })
        .collect()
}

fn build_header_map(header_row: &ExcelRow) -> HashMap<String, usize> {
    let mut headers = HashMap::new();
    for (index, value) in header_row.cells.iter().enumerate() {
        let key = normalize_header(value);
        if !key.is_empty() {
            headers.insert(key, index);
        }
    }
    headers
}

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

fn find_required_header(headers: &HashMap<String, usize>, candidates: &[&str]) -> Result<usize, AppError> {
    find_optional_header(headers, candidates).ok_or_else(|| {
        AppError::Processing(format!(
            "Missing required header. Accepted names: {}",
            candidates.join(", ")
        ))
    })
}

fn find_optional_header(headers: &HashMap<String, usize>, candidates: &[&str]) -> Option<usize> {
    candidates.iter().find_map(|name| headers.get(*name).copied())
}

fn get_required_by_index<'a>(
    row: &'a ExcelRow,
    index: usize,
    row_number: usize,
    field_name: &str,
) -> Result<&'a str, AppError> {
    let value = row
        .cells
        .get(index)
        .map(|cell| cell.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::Processing(format!("Missing {field_name} at row {row_number}, column {}", index + 1))
        })?;

    Ok(value)
}

fn get_optional_by_index(row: &ExcelRow, index: usize) -> Option<String> {
    row.cells.get(index).and_then(|cell| {
        let trimmed = cell.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}
