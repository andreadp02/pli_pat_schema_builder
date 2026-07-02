use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::repository::customer::{self, NewCustomer};
use crate::repository::excel as excel_repository;
use crate::service::excel::ExcelRow;
use crate::utils::{build_header_map, find_optional_header, find_required_header, parse_i64};
use crate::AppError;

const HEADER_ROW_INDEX: usize = 0;
const DATA_START_ROW_INDEX: usize = 1;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvalidUploadRow {
    pub row_number: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AmbiguousUploadRow {
    pub row_number: usize,
    pub tax_code: i64,
    pub ordinal_number: i64,
    pub typology: String,
    pub vat_number: Option<String>,
    pub address: String,
    pub municipality_name: String,
    pub candidate_provinces: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateCustomersExcelResult {
    pub valid_rows_count: usize,
    pub ambiguous_rows: Vec<AmbiguousUploadRow>,
    pub invalid_rows: Vec<InvalidUploadRow>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvinceResolution {
    pub row_number: usize,
    pub province_name: String,
}

#[derive(Debug, Clone)]
struct ParsedUploadRow {
    row_number: usize,
    tax_code: i64,
    ordinal_number: i64,
    typology: String,
    vat_number: Option<String>,
    address: String,
    municipality_name: String,
    province_name: Option<String>,
}

#[derive(Debug)]
struct ValidationOutcome {
    ready_rows: Vec<NewCustomer>,
    ambiguous_rows: Vec<AmbiguousUploadRow>,
    invalid_rows: Vec<InvalidUploadRow>,
}

pub async fn upload_customers_excel(file_path: &Path, db_path: &Path) -> Result<String, AppError> {
    let outcome = validate_rows(file_path, db_path, None).await?;

    if !outcome.ambiguous_rows.is_empty() {
        return Err(AppError::Processing(
            "Province is missing for one or more rows. Please validate upload and select province for ambiguous rows."
                .to_string(),
        ));
    }

    let skipped = outcome.invalid_rows.len();
    let inserted = customer::create_customers_bulk(db_path.to_path_buf(), outcome.ready_rows).await?;
    Ok(format_import_summary(inserted, skipped))
}

pub async fn validate_customers_excel(
    file_path: &Path,
    db_path: &Path,
) -> Result<ValidateCustomersExcelResult, AppError> {
    let outcome = validate_rows(file_path, db_path, None).await?;

    Ok(ValidateCustomersExcelResult {
        valid_rows_count: outcome.ready_rows.len(),
        ambiguous_rows: outcome.ambiguous_rows,
        invalid_rows: outcome.invalid_rows,
    })
}

pub async fn confirm_customers_excel_upload(
    file_path: &Path,
    db_path: &Path,
    resolutions: Vec<ProvinceResolution>,
) -> Result<String, AppError> {
    let resolution_map = build_resolution_map(resolutions);
    let outcome = validate_rows(file_path, db_path, Some(&resolution_map)).await?;

    if !outcome.ambiguous_rows.is_empty() {
        let unresolved_rows = outcome
            .ambiguous_rows
            .iter()
            .map(|row| row.row_number.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(AppError::Processing(format!(
            "Missing province selection for row(s): {unresolved_rows}"
        )));
    }

    let skipped = outcome.invalid_rows.len();
    let inserted = customer::create_customers_bulk(db_path.to_path_buf(), outcome.ready_rows).await?;
    Ok(format_import_summary(inserted, skipped))
}

fn format_import_summary(inserted: usize, skipped: usize) -> String {
    if skipped == 0 {
        format!("Imported {inserted} customers successfully")
    } else {
        format!("Imported {inserted} customers successfully, skipped {skipped} invalid row(s)")
    }
}

async fn validate_rows(
    file_path: &Path,
    db_path: &Path,
    resolutions: Option<&HashMap<usize, String>>,
) -> Result<ValidationOutcome, AppError> {
    validate_file_input(file_path)?;

    let rows = excel_repository::read_excel(file_path).await?;
    if rows.len() <= DATA_START_ROW_INDEX {
        return Err(AppError::Processing(
            "Excel file must contain a header row and at least one data row".to_string(),
        ));
    }

    let (parsed_rows, mut invalid_rows) = parse_customer_rows(&rows)?;
    let file_province_map = build_file_province_map(&parsed_rows);
    let mut ready_rows = Vec::new();
    let mut ambiguous_rows = Vec::new();

    for parsed in parsed_rows {
        let province_in_row = normalize_optional_string(parsed.province_name.as_deref())
            .or_else(|| file_province_map.get(&municipality_key(&parsed.municipality_name)).cloned());

        if let Some(province_name) = province_in_row {
            ready_rows.push(NewCustomer {
                tax_code: parsed.tax_code,
                ordinal_number: parsed.ordinal_number,
                typology: parsed.typology,
                vat_number: parsed.vat_number,
                address: parsed.address,
                municipality_name: parsed.municipality_name,
                province_name,
            });
            continue;
        }

        let candidates = customer::find_provinces_by_municipality(
            db_path.to_path_buf(),
            parsed.municipality_name.clone(),
        )
        .await?;

        match candidates.len() {
            0 => invalid_rows.push(InvalidUploadRow {
                row_number: parsed.row_number,
                message: format!(
                    "Row {}: municipality '{}' not found in database and province is missing",
                    parsed.row_number, parsed.municipality_name
                ),
            }),
            1 => ready_rows.push(NewCustomer {
                tax_code: parsed.tax_code,
                ordinal_number: parsed.ordinal_number,
                typology: parsed.typology,
                vat_number: parsed.vat_number,
                address: parsed.address,
                municipality_name: parsed.municipality_name,
                province_name: candidates[0].clone(),
            }),
            _ => {
                let selected_province = resolutions
                    .and_then(|map| map.get(&parsed.row_number))
                    .and_then(|value| normalize_optional_string(Some(value.as_str())));

                if let Some(selected_province) = selected_province {
                    if let Some(candidate) = candidates
                        .iter()
                        .find(|candidate| candidate.eq_ignore_ascii_case(&selected_province))
                    {
                        ready_rows.push(NewCustomer {
                            tax_code: parsed.tax_code,
                            ordinal_number: parsed.ordinal_number,
                            typology: parsed.typology,
                            vat_number: parsed.vat_number,
                            address: parsed.address,
                            municipality_name: parsed.municipality_name,
                            province_name: candidate.clone(),
                        });
                    } else {
                        invalid_rows.push(InvalidUploadRow {
                            row_number: parsed.row_number,
                            message: format!(
                                "Row {}: selected province '{}' is not valid for municipality '{}'",
                                parsed.row_number, selected_province, parsed.municipality_name
                            ),
                        });
                    }
                } else {
                    ambiguous_rows.push(AmbiguousUploadRow {
                        row_number: parsed.row_number,
                        tax_code: parsed.tax_code,
                        ordinal_number: parsed.ordinal_number,
                        typology: parsed.typology,
                        vat_number: parsed.vat_number,
                        address: parsed.address,
                        municipality_name: parsed.municipality_name,
                        candidate_provinces: candidates,
                    });
                }
            }
        }
    }

    Ok(ValidationOutcome {
        ready_rows,
        ambiguous_rows,
        invalid_rows,
    })
}

fn validate_file_input(file_path: &Path) -> Result<(), AppError> {
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

    Ok(())
}

fn parse_customer_rows(
    rows: &[ExcelRow],
) -> Result<(Vec<ParsedUploadRow>, Vec<InvalidUploadRow>), AppError> {
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
    let typology_idx =
        find_required_header(&headers, &["typology", "tipologia", "tipologia_punto_vendita"])?;
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
    let address_idx =
        find_required_header(&headers, &["address", "indirizzo", "indirizzo_punto_vendita"])?;
    let province_name_idx =
        find_optional_header(&headers, &["provincia", "nome_provincia", "province_name"]);
    let municipality_name_idx =
        find_required_header(&headers, &["comune", "nome_comune", "comune_punto_vendita"])?;

    let mut parsed_rows = Vec::new();
    let mut invalid_rows = Vec::new();

    for (row_index, row) in rows.iter().enumerate().skip(DATA_START_ROW_INDEX) {
        let row_number = row_index + 1;

        // calamine's used range runs to the sheet's stored dimension, which for ADM exports extends
        // past the real data with trailing rows that carry stray cells but no tax_code. A row without
        // a tax_code (the customer key) is not importable, so skip it — same as product import skipping
        // rows without `gruppo`.
        if get_optional_by_index(row, tax_code_idx).is_none() {
            continue;
        }

        match parse_customer_row(
            row,
            row_number,
            tax_code_idx,
            ordinal_number_idx,
            typology_idx,
            vat_idx,
            address_idx,
            municipality_name_idx,
            province_name_idx,
        ) {
            Ok(parsed) => parsed_rows.push(parsed),
            Err(error) => invalid_rows.push(InvalidUploadRow {
                row_number,
                message: error.to_string(),
            }),
        }
    }

    Ok((parsed_rows, invalid_rows))
}

#[allow(clippy::too_many_arguments)]
fn parse_customer_row(
    row: &ExcelRow,
    row_number: usize,
    tax_code_idx: usize,
    ordinal_number_idx: usize,
    typology_idx: usize,
    vat_idx: Option<usize>,
    address_idx: usize,
    municipality_name_idx: usize,
    province_name_idx: Option<usize>,
) -> Result<ParsedUploadRow, AppError> {
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

    Ok(ParsedUploadRow {
        row_number,
        tax_code,
        ordinal_number,
        typology: validate_typology(
            get_required_by_index(row, typology_idx, row_number, "typology")?,
            row_number,
        )?,
        vat_number: vat_idx.and_then(|idx| get_optional_by_index(row, idx)),
        address: get_required_by_index(row, address_idx, row_number, "address")?.to_string(),
        municipality_name: get_required_by_index(
            row,
            municipality_name_idx,
            row_number,
            "municipality_name",
        )?
        .to_string(),
        province_name: province_name_idx.and_then(|idx| get_optional_by_index(row, idx)),
    })
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
            AppError::Processing(format!(
                "Missing {field_name} at row {row_number}, column {}",
                index + 1
            ))
        })?;

    Ok(value)
}

// Validate against the enum at parse time so a bad typology is skipped+reported, not left to blow up
// the all-or-nothing bulk insert (customer.typology has a CHECK constraint).
fn validate_typology(value: &str, row_number: usize) -> Result<String, AppError> {
    let normalized = value.trim().to_uppercase();
    match normalized.as_str() {
        customer::TYPOLOGY_ESERCIZIO_DI_VICINATO
        | customer::TYPOLOGY_RIVENDITA
        | customer::TYPOLOGY_FARMACIA
        | customer::TYPOLOGY_PARAFARMACIA => Ok(normalized),
        _ => Err(AppError::Processing(format!(
            "Row {row_number}: invalid typology '{value}'"
        ))),
    }
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

fn normalize_optional_string(value: Option<&str>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

// ponytail: first-writer-wins if the same municipality gets two different provinces in the file
fn build_file_province_map(parsed_rows: &[ParsedUploadRow]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for parsed in parsed_rows {
        if let Some(province) = normalize_optional_string(parsed.province_name.as_deref()) {
            map.entry(municipality_key(&parsed.municipality_name))
                .or_insert(province);
        }
    }
    map
}

fn municipality_key(municipality_name: &str) -> String {
    municipality_name.trim().to_lowercase()
}

fn build_resolution_map(resolutions: Vec<ProvinceResolution>) -> HashMap<usize, String> {
    let mut map = HashMap::new();
    for resolution in resolutions {
        map.insert(resolution.row_number, resolution.province_name);
    }
    map
}

#[cfg(test)]
#[path = "customer_tests.rs"]
mod tests;
