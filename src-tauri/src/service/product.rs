use std::collections::HashSet;
use std::path::Path;

use crate::repository::excel as excel_repository;
use crate::repository::product::{self, NewProduct, ProductType, SkeletonUpdate};
use crate::service::excel::ExcelRow;
use crate::utils::{build_header_map, find_required_header, parse_i64};
use crate::AppError;

const HEADER_ROW_INDEX: usize = 0;
const DATA_START_ROW_INDEX: usize = 1;
const PLI_GRUPPO_VALUE: i64 = 5;
const INSERT_BATCH_SIZE: usize = 200;

/// Columns located by header name rather than a fixed position: exports differ in column
/// order/count (e.g. the raw "codici" export has extra leading columns the reduced one doesn't).
struct ProductColumns {
    code: usize,   // Prodotto (product code)
    info3: usize,  // PLI units / PAT packages
    info4: usize,  // PAT units
    gruppo: usize, // 5 = PLI, other number = PAT, empty = skip
}

impl ProductColumns {
    fn detect(header_row: &ExcelRow) -> Result<Self, AppError> {
        let headers = build_header_map(header_row);
        Ok(Self {
            code: find_required_header(&headers, &["prodotto", "codice_prodotto"])?,
            info3: find_required_header(&headers, &["info_3"])?,
            info4: find_required_header(&headers, &["info_4"])?,
            gruppo: find_required_header(&headers, &["gruppo"])?,
        })
    }
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

    let new_products = parse_products_rows(&rows)?;

    let inserted = product::create_products_in_batches(
        db_path.to_path_buf(),
        new_products,
        INSERT_BATCH_SIZE,
    )
    .await?;

    Ok(format!("Imported {inserted} products successfully"))
}

fn parse_products_rows(rows: &[ExcelRow]) -> Result<Vec<NewProduct>, AppError> {
    let header_row = rows
        .get(HEADER_ROW_INDEX)
        .ok_or_else(|| AppError::Processing("Missing header row".to_string()))?;
    let columns = ProductColumns::detect(header_row)?;

    let mut products = Vec::new();
    for (row_index, row) in rows.iter().enumerate().skip(DATA_START_ROW_INDEX) {
        if let Some(product) = parse_product_row(row_index, row, &columns)? {
            products.push(product);
        }
    }
    Ok(products)
}

/// Parses one info3/info4 row into a `NewProduct`, or `None` when the row has no numeric `gruppo`
/// (header leftovers / blank lines are not products). Description, capacity and nicotine are left
/// as placeholders here — they are filled later from the skeleton files. The PLI `adm_code` is the
/// exception: it's a pure function of `code` (see `pli_adm_code`), so it's derived here rather than
/// waiting for the skeleton.
fn parse_product_row(
    row_index: usize,
    row: &ExcelRow,
    columns: &ProductColumns,
) -> Result<Option<NewProduct>, AppError> {
    let row_number = row_index + 1;

    let Some(gruppo) = optional_cell(row, columns.gruppo).and_then(|v| v.parse::<i64>().ok())
    else {
        return Ok(None);
    };

    let code = get_required_cell(row, columns.code, row_number, "code")?.to_string();

    if gruppo == PLI_GRUPPO_VALUE {
        let units = parse_non_negative_u32(
            get_required_cell(row, columns.info3, row_number, "units")?,
            row_number,
            "units",
        )?;
        let adm_code = pli_adm_code(&code);
        Ok(Some(NewProduct {
            product_type: ProductType::Pli,
            code,
            description: String::new(),
            units,
            // capacity/nicotine are skeleton-owned; left NULL until a skeleton is uploaded.
            capacity: None,
            nicotine: None,
            packages: None,
            adm_code,
            tabella: None,
        }))
    } else {
        let units = parse_non_negative_u32(
            get_required_cell(row, columns.info4, row_number, "units")?,
            row_number,
            "units",
        )?;
        let packages = parse_non_negative_u32(
            get_required_cell(row, columns.info3, row_number, "packages")?,
            row_number,
            "packages",
        )?;
        Ok(Some(NewProduct {
            product_type: ProductType::Pat,
            code,
            description: String::new(),
            units,
            capacity: None,
            nicotine: None,
            packages: Some(packages),
            adm_code: None,
            // tabella is skeleton-owned (read from skeleton_pat); gruppo only classifies the type.
            tabella: None,
        }))
    }
}

fn optional_cell(row: &ExcelRow, column_index: usize) -> Option<&str> {
    row.cells
        .get(column_index)
        .map(|cell| cell.trim())
        .filter(|cell| !cell.is_empty())
}

/// Second product phase: read a skeleton file (skeleton_pli or skeleton_pat) and fill the
/// skeleton-owned fields on products already imported, matched by product code.
pub async fn upload_skeleton_excel(file_path: &Path, db_path: &Path) -> Result<String, AppError> {
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

    let header_index = find_skeleton_header(&rows).ok_or_else(|| {
        AppError::Processing(
            "Could not find the product table header (expected 'Codice prodotto' and 'Denominazione prodotto')"
                .to_string(),
        )
    })?;
    let layout = SkeletonLayout::detect(&rows[header_index])?;
    let updates = parse_skeleton_rows(&rows, header_index + 1, &layout);

    if updates.is_empty() {
        return Err(AppError::Processing(
            "No product rows found in the skeleton file".to_string(),
        ));
    }

    let total = updates.len();
    let matched = product::update_products_from_skeleton(db_path.to_path_buf(), updates).await?;
    let not_found = total.saturating_sub(matched);

    Ok(format!("Updated {matched} products ({not_found} codes not found)"))
}

enum SkeletonLayout {
    Pli {
        code: usize,
        description: usize,
        capacity: usize,
        nicotine: usize,
    },
    Pat {
        code: usize,
        description: usize,
        adm_code: Option<usize>,
        tabella: Option<usize>,
    },
}

impl SkeletonLayout {
    fn detect(header: &ExcelRow) -> Result<SkeletonLayout, AppError> {
        let norm: Vec<String> = header.cells.iter().map(|c| normalize_skeleton(c)).collect();
        let code_indices: Vec<usize> = norm
            .iter()
            .enumerate()
            .filter(|(_, h)| h.contains("codice prodotto"))
            .map(|(i, _)| i)
            .collect();
        // Prefer the exact 'Denominazione prodotto' column; PAT skeletons carry other
        // 'Denominazione …' columns (e.g. tabella) that must not be mistaken for it.
        let description = norm
            .iter()
            .position(|h| h.contains("denominazione prodotto"))
            .or_else(|| norm.iter().position(|h| h.contains("denominazione")))
            .ok_or_else(|| {
                AppError::Processing("Skeleton missing 'Denominazione prodotto' column".to_string())
            })?;
        let missing_code =
            || AppError::Processing("Skeleton missing 'Codice prodotto' column".to_string());

        let capacity = norm.iter().position(|h| h.contains("capacit"));
        let nicotine = norm.iter().position(|h| h.contains("nicotin"));

        if let (Some(capacity), Some(nicotine)) = (capacity, nicotine) {
            // PLI skeleton: one code column (= DB code) plus capacity + nicotine.
            let code = *code_indices.first().ok_or_else(missing_code)?;
            Ok(SkeletonLayout::Pli {
                code,
                description,
                capacity,
                nicotine,
            })
        } else {
            // PAT skeleton: the rightmost 'Codice prodotto' is the company code (= DB code);
            // the leftmost one, when present, is the ADM code written to tracciati_pat col L.
            let code = *code_indices.last().ok_or_else(missing_code)?;
            let adm_code = (code_indices.len() >= 2).then(|| code_indices[0]);
            let tabella = norm.iter().position(|h| h.contains("tabella"));
            Ok(SkeletonLayout::Pat {
                code,
                description,
                adm_code,
                tabella,
            })
        }
    }
}

fn parse_skeleton_rows(
    rows: &[ExcelRow],
    start_index: usize,
    layout: &SkeletonLayout,
) -> Vec<SkeletonUpdate> {
    let mut updates = Vec::new();
    // The skeleton may list the same product on several rows; keep only the first occurrence so the
    // matched/not-found counts aren't inflated and the same product isn't updated twice. The key
    // matches the repository's normalization (trim + uppercase).
    let mut seen: HashSet<String> = HashSet::new();
    for row in rows.iter().skip(start_index) {
        match layout {
            SkeletonLayout::Pli {
                code,
                description,
                capacity,
                nicotine,
            } => {
                let Some(product_code) = optional_cell(row, *code) else {
                    continue;
                };
                if !seen.insert(product_code.trim().to_uppercase()) {
                    continue;
                }
                updates.push(SkeletonUpdate {
                    code: product_code.to_string(),
                    description: optional_cell(row, *description).unwrap_or_default().to_string(),
                    capacity: optional_cell(row, *capacity).and_then(parse_f64_opt),
                    nicotine: optional_cell(row, *nicotine).and_then(parse_f64_opt),
                    // adm_code is import-owned for PLI (derived from `code`, see pli_adm_code).
                    adm_code: None,
                    tabella: None,
                });
            }
            SkeletonLayout::Pat {
                code,
                description,
                adm_code,
                tabella,
            } => {
                let Some(product_code) = optional_cell(row, *code) else {
                    continue;
                };
                if !seen.insert(product_code.trim().to_uppercase()) {
                    continue;
                }
                updates.push(SkeletonUpdate {
                    code: product_code.to_string(),
                    description: optional_cell(row, *description).unwrap_or_default().to_string(),
                    capacity: None,
                    nicotine: None,
                    adm_code: adm_code
                        .and_then(|idx| optional_cell(row, idx))
                        .map(|s| s.to_string()),
                    tabella: tabella
                        .and_then(|idx| optional_cell(row, idx))
                        .and_then(|s| s.parse::<i64>().ok()),
                });
            }
        }
    }
    updates
}

fn find_skeleton_header(rows: &[ExcelRow]) -> Option<usize> {
    rows.iter().position(|row| {
        let has_code = row
            .cells
            .iter()
            .any(|c| normalize_skeleton(c).contains("codice prodotto"));
        let has_description = row
            .cells
            .iter()
            .any(|c| normalize_skeleton(c).contains("denominazione"));
        has_code && has_description
    })
}

/// Lowercase and collapse whitespace/newlines so header matching is resilient (accents are kept,
/// but we only test prefixes like "capacit"/"nicotin" that precede them).
fn normalize_skeleton(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// PLI tracciato codes drop a single trailing D/K/S that follows the numeric part
/// ("PL0012162D" → "PL0012162"), stored as the PLI `adm_code`. Returns `None` when the rule
/// doesn't apply, so COALESCE leaves the column untouched.
fn pli_adm_code(code: &str) -> Option<String> {
    let trimmed = code.trim();
    let mut chars = trimmed.chars().rev();
    let last = chars.next()?;
    if !matches!(last.to_ascii_uppercase(), 'D' | 'K' | 'S') || !chars.next()?.is_ascii_digit() {
        return None;
    }
    Some(trimmed[..trimmed.len() - 1].to_string()) // last is ASCII D/K/S → 1 byte
}

fn parse_f64_opt(value: &str) -> Option<f64> {
    value.replace(',', ".").parse::<f64>().ok().filter(|f| *f >= 0.0)
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

#[cfg(test)]
mod tests {
    use super::{parse_f64_opt, parse_product_row, parse_products_rows, pli_adm_code, ProductColumns};
    use crate::service::excel::ExcelRow;

    fn row(cells: &[&str]) -> ExcelRow {
        ExcelRow {
            cells: cells.iter().map(|c| c.to_string()).collect(),
        }
    }

    // Mirrors the 11-column "codici" export layout (extra tipo/DocTip/.../PrdQta columns before
    // Prodotto/Info 3/Info 4/gruppo), so field indices don't shift when written by hand here.
    fn eleven_column_layout() -> ProductColumns {
        ProductColumns {
            code: 5,
            info3: 8,
            info4: 9,
            gruppo: 10,
        }
    }

    #[test]
    fn parses_capacity_with_dot_or_comma() {
        assert_eq!(parse_f64_opt("2.5"), Some(2.5));
        assert_eq!(parse_f64_opt("2,5"), Some(2.5));
        assert_eq!(parse_f64_opt("10"), Some(10.0));
        assert_eq!(parse_f64_opt("-1"), None);
        assert_eq!(parse_f64_opt("abc"), None);
    }

    #[test]
    fn strips_trailing_dks_after_digit_only() {
        assert_eq!(pli_adm_code("PL0012162D").as_deref(), Some("PL0012162"));
        assert_eq!(pli_adm_code("PL0012162K").as_deref(), Some("PL0012162"));
        assert_eq!(pli_adm_code("PL0012162s").as_deref(), Some("PL0012162"));
        assert_eq!(pli_adm_code("PL0012162"), None); // no trailing letter
        assert_eq!(pli_adm_code("PLN011954"), None); // ends with digit
        assert_eq!(pli_adm_code("PLD"), None); // D not preceded by a digit
    }

    // The PLI adm_code is derived from the code at import time, not left for the skeleton to fill.
    // PAT never gets one here — it stays skeleton-owned.
    #[test]
    fn import_derives_pli_adm_code_and_leaves_pat_unset() {
        let columns = eleven_column_layout();

        let pli = row(&["", "", "", "", "", "PL0012162D", "", "", "10", "", "5"]);
        assert_eq!(
            parse_product_row(1, &pli, &columns).unwrap().unwrap().adm_code.as_deref(),
            Some("PL0012162")
        );

        let pli_no_suffix = row(&["", "", "", "", "", "PL0012162", "", "", "10", "", "5"]);
        assert_eq!(
            parse_product_row(1, &pli_no_suffix, &columns).unwrap().unwrap().adm_code,
            None
        );

        let pat = row(&["", "", "", "", "", "A1", "", "", "2", "3", "7"]);
        assert_eq!(parse_product_row(1, &pat, &columns).unwrap().unwrap().adm_code, None);
    }

    // A re-import with a changed info3 cell must reparse to the new units/packages value —
    // the DB-level UPSERT (repository::product tests) then overwrites only this import-owned field.
    #[test]
    fn reparsing_a_row_with_a_changed_info3_value_updates_units_and_packages() {
        let columns = eleven_column_layout();

        // PLI: F=code, I=info3 (units), K=gruppo=5.
        let pli = |info3: &str| row(&["", "", "", "", "", "P1", "", "", info3, "", "5"]);
        assert_eq!(parse_product_row(1, &pli("10"), &columns).unwrap().unwrap().units, 10);
        assert_eq!(parse_product_row(1, &pli("25"), &columns).unwrap().unwrap().units, 25);

        // PAT: F=code, I=info3 (packages), J=info4 (units), K=gruppo!=5.
        let pat = |info3: &str| row(&["", "", "", "", "", "A1", "", "", info3, "3", "7"]);
        assert_eq!(parse_product_row(1, &pat("2"), &columns).unwrap().unwrap().packages, Some(2));
        assert_eq!(parse_product_row(1, &pat("9"), &columns).unwrap().unwrap().packages, Some(9));
    }

    // Real ADM exports come in two shapes: a reduced 5-column sheet and the raw "codici" export
    // with 6 extra leading columns. Both must resolve by header name to the same fields.
    #[test]
    fn parses_both_real_export_layouts_by_header_name() {
        let reduced = vec![
            row(&["Prodotto", "Descrizione", "Info 3", "Info 4", "gruppo"]),
            row(&["PL000", "LEM PLUS DEVICE + POD", "1", "1", "5"]),
        ];
        let products = parse_products_rows(&reduced).unwrap();
        assert_eq!(products.len(), 1);
        assert_eq!((products[0].code.as_str(), products[0].units), ("PL000", 1));

        let raw_codici = vec![
            row(&[
                "tipo", "DocTip", "DocNum", "AnaType", "AnaCod", "Prodotto", "Descrizione", "PrdQta",
                "Info 3", "Info 4", "gruppo",
            ]),
            row(&[
                "3", "Giacenza finale", "", "", "", "BOXSEF18", "EASY KIT", "27", "18", "64", "4",
            ]),
        ];
        let products = parse_products_rows(&raw_codici).unwrap();
        assert_eq!(products.len(), 1);
        assert_eq!(
            (products[0].code.as_str(), products[0].units, products[0].packages),
            ("BOXSEF18", 64, Some(18))
        );
    }
}
