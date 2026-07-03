use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::repository::customer::{self as customer_repository, Customer};
use crate::repository::excel::{self as excel_repository, CellValue, TemplateCell};
use crate::repository::product::{self as product_repository, Product, ProductType};
use crate::service::invoice::{self, Invoice, InvoiceLine};
use crate::service::settings::{self, AccisaCoefficients};
use crate::AppError;

/// Represents a single row from an Excel sheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelRow {
    pub cells: Vec<String>,
}

/// Paths to the two generated files plus any per-invoice/-line problems collected along the way.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateResult {
    pub tracciati_pli: String,
    pub tracciati_pat: String,
    pub warnings: Vec<String>,
}

const RIVENDITA: &str = "RIVENDITA";

// tracciati_pli: sheet + first data row (the template's prototype row carries constants/formulas).
pub const PLI_SHEET: &str = "MENSILE PUNTI VENDITA RIFORNITI";
const PLI_START_ROW: u32 = 6;
// tracciati_pat
pub const PAT_SHEET: &str = "PROSPETTO IMMISSIONI IN CONSUMO";
const PAT_START_ROW: u32 = 2;

/// Reads every invoice, matches each to a customer and its excise lines to products, and fills the
/// two saved templates. Any problem with an invoice (bad number, missing customer, missing/incomplete
/// product) drops the whole invoice — none of its rows reach the output — and is collected as a
/// warning rather than aborting the run.
pub async fn generate_tracciati(
    invoice_paths: Vec<String>,
    fortnight_end: String,
    pli_template: &Path,
    pat_template: &Path,
    output_dir: &Path,
    db_path: &Path,
) -> Result<GenerateResult, AppError> {
    if invoice_paths.is_empty() {
        return Err(AppError::Processing("No invoices selected".to_string()));
    }

    // PAT writes the fortnight end as a real date; PLI writes only its month ("MM/YYYY").
    let (year, month, day) = parse_fortnight_end(&fortnight_end)?;
    let pli_period = format!("{month:02}/{year:04}");
    let (period_start, period_end) = fortnight_range(year, month, day);

    // Excise coefficients embedded into the accisa formulas (user-editable in the Template page).
    let coeffs = settings::get_accisa_coefficients(db_path).await?;

    let mut pli_rows: Vec<Vec<TemplateCell>> = Vec::new();
    let mut pat_rows: Vec<Vec<TemplateCell>> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    for path in &invoice_paths {
        let invoice = match invoice::parse_invoice(Path::new(path)).await {
            Ok(invoice) => invoice,
            Err(e) => {
                warnings.push(format!("{path}: {e}"));
                continue;
            }
        };

        if invoice.date < period_start || invoice.date > period_end {
            let (y, m, d) = invoice.date;
            warnings.push(format!(
                "Invoice {}: date {d:02}/{m:02}/{y:04} outside selected period (skipped)",
                invoice.number
            ));
            continue;
        }

        let Some(customer) = resolve_customer(db_path, &invoice).await? else {
            warnings.push(format!(
                "Invoice {}: customer not found (CF '{}', P.IVA '{}')",
                invoice.number, invoice.fiscal_code, invoice.vat
            ));
            continue;
        };

        // Buffer this invoice's rows: commit them only if every line is clean, so a single bad line
        // drops the whole invoice from the output.
        let mut inv_pli: Vec<Vec<TemplateCell>> = Vec::new();
        let mut inv_pat: Vec<Vec<TemplateCell>> = Vec::new();
        let mut problem: Option<String> = None;

        for line in &invoice.lines {
            let product = product_repository::get_product_by_code(
                db_path.to_path_buf(),
                line.code.clone(),
                None,
            )
            .await?;

            let Some(product) = product else {
                problem = Some(format!(
                    "Invoice {}: product '{}' is not in the products database (invoice skipped)",
                    invoice.number, line.code
                ));
                break;
            };

            if !product.is_skeleton_complete() {
                problem = Some(format!(
                    "Invoice {}: product '{}' has no skeleton data (invoice skipped)",
                    invoice.number, line.code
                ));
                break;
            }

            match product.product_type {
                ProductType::Pli => inv_pli.push(build_pli_row(
                    &customer, &product, &invoice, line, &pli_period, &coeffs,
                )),
                ProductType::Pat => inv_pat.push(build_pat_row(
                    &customer, &product, &invoice, line, (year, month, day), &coeffs,
                )),
            }
        }

        if let Some(warning) = problem {
            warnings.push(warning);
            continue;
        }
        pli_rows.extend(inv_pli);
        pat_rows.extend(inv_pat);
    }

    let pli_output = output_dir.join(format!("tracciati_pli_{year:04}-{month:02}-{day:02}.xlsx"));
    let pat_output = output_dir.join(format!("tracciati_pat_{year:04}-{month:02}-{day:02}.xlsx"));

    excel_repository::fill_template(pli_template, &pli_output, PLI_SHEET, PLI_START_ROW, pli_rows)
        .await?;
    excel_repository::fill_template(pat_template, &pat_output, PAT_SHEET, PAT_START_ROW, pat_rows)
        .await?;

    Ok(GenerateResult {
        tracciati_pli: pli_output.to_string_lossy().into_owned(),
        tracciati_pat: pat_output.to_string_lossy().into_owned(),
        warnings,
    })
}

/// Customer match: try the invoice fiscal code first, then its VAT number, both against
/// `customer.vat_number` (the single CF/PIVA stored per customer).
async fn resolve_customer(db_path: &Path, invoice: &Invoice) -> Result<Option<Customer>, AppError> {
    for identifier in [&invoice.fiscal_code, &invoice.vat] {
        if identifier.is_empty() {
            continue;
        }
        let found = customer_repository::get_customer_by_vat_number(
            db_path.to_path_buf(),
            identifier.clone(),
        )
        .await?;
        if found.is_some() {
            return Ok(found);
        }
    }
    Ok(None)
}

fn cell(column: u32, value: impl Into<String>) -> TemplateCell {
    TemplateCell {
        column,
        value: CellValue::Text(value.into()),
    }
}

/// Forces an Excel string cell so a numeric-looking value (a P.IVA with a leading zero) keeps its
/// exact text instead of being coerced to a number.
fn text_cell(column: u32, value: impl Into<String>) -> TemplateCell {
    TemplateCell {
        column,
        value: CellValue::Str(value.into()),
    }
}

fn date_cell(column: u32, year: i32, month: i32, day: i32) -> TemplateCell {
    TemplateCell {
        column,
        value: CellValue::Date { year, month, day },
    }
}

/// An Excel formula cell; row references are written against the sheet's data start row and bumped
/// per output row on write (see `repository::excel::fill_template`).
fn formula_cell(column: u32, formula: impl Into<String>) -> TemplateCell {
    TemplateCell {
        column,
        value: CellValue::Formula(formula.into()),
    }
}

/// The PLI excise coefficient: zero-nicotine liquids (code `PLN…`) use their own rate, everything
/// else the standard PLI rate.
fn pli_accisa_coeff(code: &str, coeffs: &AccisaCoefficients) -> f64 {
    if code.starts_with("PLN") {
        coeffs.pli_pln
    } else {
        coeffs.pli_pl
    }
}

/// Parses the fortnight end the frontend sends as an ISO date ("YYYY-MM-DD").
fn parse_fortnight_end(iso: &str) -> Result<(i32, i32, i32), AppError> {
    let parts: Vec<&str> = iso.split('-').collect();
    let bad = || AppError::Processing(format!("Invalid fortnight end date '{iso}'"));
    let [y, m, d] = parts.as_slice() else {
        return Err(bad());
    };
    Ok((
        y.parse().map_err(|_| bad())?,
        m.parse().map_err(|_| bad())?,
        d.parse().map_err(|_| bad())?,
    ))
}

/// The selected fortnight, inclusive: day 1..15 for the first half, 16..end-of-month for the second
/// (the end day is the last day of the month, as the frontend proposes). Invoices dated outside are
/// dropped. Tuples are (year, month, day) so plain comparison orders them correctly.
fn fortnight_range(year: i32, month: i32, end_day: i32) -> ((i32, i32, i32), (i32, i32, i32)) {
    let start_day = if end_day <= 15 { 1 } else { 16 };
    ((year, month, start_day), (year, month, end_day))
}

fn build_pli_row(
    customer: &Customer,
    product: &Product,
    invoice: &Invoice,
    line: &InvoiceLine,
    pli_period: &str,
    coeffs: &AccisaCoefficients,
) -> Vec<TemplateCell> {
    let tax_code = customer.tax_code.to_string();
    // Rivendita → CMNR (F); otherwise the esercizio-vicinato number (E).
    let (esercizio, cmnr) = if customer.typology == RIVENDITA {
        (String::new(), tax_code)
    } else {
        (tax_code, String::new())
    };
    let confezioni = i64::from(product.units) * line.quantity;
    let product_code = product
        .adm_code
        .clone()
        .filter(|c| !c.is_empty())
        .unwrap_or_else(|| product.code.clone());

    vec![
        cell(4, pli_period.to_string()),                           // D Data mese (MM/YYYY)
        cell(5, esercizio),                                        // E Numero esercizio vicinato
        cell(6, cmnr),                                             // F CMNR rivendita
        cell(7, customer.ordinal_number.to_string()),             // G Numero ordinale
        cell(8, customer.municipality_name.clone()),              // H Comune
        cell(9, customer.province_name.clone()),                  // I Provincia
        text_cell(10, customer.vat_number.clone().unwrap_or_default()), // J CF/P.IVA
        cell(11, product.description.clone()),                    // K Denominazione
        cell(12, product_code),                                   // L Codice prodotto (ADM se impostato)
        cell(13, product.capacity.unwrap_or(0).to_string()),      // M Capacità
        cell(14, product.nicotine.unwrap_or(0).to_string()),      // N Nicotina
        cell(15, confezioni.to_string()),                         // O Numero di confezioni (B)
        // P Quantità totale (Litri) = template formula
        cell(17, invoice.number.to_string()),                     // Q N. Fattura (working)
        formula_cell(                                             // R Accisa = O*(M*coefficiente)
            18,
            format!(
                "O{PLI_START_ROW}*(M{PLI_START_ROW}*{})",
                pli_accisa_coeff(&product.code, coeffs)
            ),
        ),
    ]
}

fn build_pat_row(
    customer: &Customer,
    product: &Product,
    invoice: &Invoice,
    line: &InvoiceLine,
    (year, month, day): (i32, i32, i32),
    coeffs: &AccisaCoefficients,
) -> Vec<TemplateCell> {
    let cmnr = if customer.typology == RIVENDITA {
        customer.tax_code.to_string()
    } else {
        String::new()
    };
    let confezioni = i64::from(product.packages.unwrap_or(0)) * line.quantity;

    vec![
        date_cell(5, year, month, day),                            // E Data fine quindicina
        cell(6, cmnr),                                             // F CMNR rivendita
        cell(7, customer.ordinal_number.to_string()),             // G Numero ordinale
        cell(8, customer.municipality_name.clone()),              // H Comune
        cell(9, customer.province_name.clone()),                  // I Provincia
        text_cell(10, customer.vat_number.clone().unwrap_or_default()), // J CF/P.IVA
        cell(11, product.tabella.map(|t| t.to_string()).unwrap_or_default()), // K Tabella
        cell(12, product.adm_code.clone().unwrap_or_default()),   // L Codice prodotto (ADM)
        cell(13, product.description.clone()),                    // M Denominazione
        cell(14, product.units.to_string()),                      // N N° pezzi per confezione
        cell(15, confezioni.to_string()),                         // O N° confezioni immesse (B)
        // P N° totale pezzi (C=A*B) = template formula
        cell(17, product.code.clone()),                           // Q codice (working)
        cell(18, invoice.number.to_string()),                     // R N. Fattura (working)
        formula_cell(19, format!("{}*P{PAT_START_ROW}", coeffs.pat)), // S Accisa = coefficiente*P
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fortnight_range_splits_and_bounds_inclusively() {
        // First half: 15 → [1, 15].
        let (start, end) = fortnight_range(2026, 6, 15);
        assert_eq!(start, (2026, 6, 1));
        assert_eq!(end, (2026, 6, 15));
        assert!((2026, 6, 1) >= start && (2026, 6, 15) <= end); // endpoints included
        assert!((2026, 5, 31) < start); // previous month out
        assert!((2026, 6, 16) > end); // next fortnight out

        // Second half: end-of-month 30 → [16, 30].
        let (start, _) = fortnight_range(2026, 6, 30);
        assert_eq!(start, (2026, 6, 16));
        assert!((2026, 6, 15) < start); // first fortnight out
    }

    #[test]
    fn pli_accisa_coeff_splits_on_pln_prefix() {
        let coeffs = AccisaCoefficients { pli_pln: 0.05, pli_pl: 0.124672, pat: 0.0036 };
        // Zero-nicotine (PLN…) → its own rate; everything else → standard PLI rate.
        assert_eq!(pli_accisa_coeff("PLN0012162", &coeffs), 0.05);
        assert_eq!(pli_accisa_coeff("PL0012162D", &coeffs), 0.124672);
    }
}
