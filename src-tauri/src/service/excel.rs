use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::repository::customer::{self as customer_repository, Customer};
use crate::repository::excel::{self as excel_repository, TemplateCell};
use crate::repository::product::{self as product_repository, Product, ProductType};
use crate::service::invoice::{self, Invoice, InvoiceLine};
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
/// two saved templates. Missing customers/products and bad invoice numbers are collected as
/// warnings rather than aborting the whole run.
pub async fn generate_tracciati(
    invoice_paths: Vec<String>,
    period: String,
    pli_template: &Path,
    pat_template: &Path,
    output_dir: &Path,
    db_path: &Path,
) -> Result<GenerateResult, AppError> {
    if invoice_paths.is_empty() {
        return Err(AppError::Processing("No invoices selected".to_string()));
    }

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

        let Some(customer) = resolve_customer(db_path, &invoice).await? else {
            warnings.push(format!(
                "Invoice {}: customer not found (CF '{}', P.IVA '{}')",
                invoice.number, invoice.fiscal_code, invoice.vat
            ));
            continue;
        };

        for line in &invoice.lines {
            let product = product_repository::get_product_by_code(
                db_path.to_path_buf(),
                line.code.clone(),
                None,
            )
            .await?;

            let Some(product) = product else {
                // An excise line whose product is not in the DB is a hard error: abort with no
                // output so the operator can fix the products table first.
                return Err(AppError::Processing(format!(
                    "Product '{}' (invoice {}) is not in the products database — customer P.IVA '{}', C.F. '{}'. No files were generated.",
                    line.code,
                    invoice.number,
                    customer.vat_number.as_deref().unwrap_or("-"),
                    customer.tax_code
                )));
            };

            if !product.is_skeleton_complete() {
                warnings.push(format!(
                    "Invoice {}: product '{}' has no skeleton data (skipped)",
                    invoice.number, line.code
                ));
                continue;
            }

            match product.product_type {
                ProductType::Pli => {
                    pli_rows.push(build_pli_row(&customer, &product, &invoice, line, &period))
                }
                ProductType::Pat => {
                    pat_rows.push(build_pat_row(&customer, &product, &invoice, line, &period))
                }
            }
        }
    }

    let pli_output = output_dir.join("tracciati_pli.xlsx");
    let pat_output = output_dir.join("tracciati_pat.xlsx");

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
        value: value.into(),
    }
}

fn build_pli_row(
    customer: &Customer,
    product: &Product,
    invoice: &Invoice,
    line: &InvoiceLine,
    period: &str,
) -> Vec<TemplateCell> {
    let tax_code = customer.tax_code.to_string();
    // Rivendita → CMNR (F); otherwise the esercizio-vicinato number (E).
    let (esercizio, cmnr) = if customer.typology == RIVENDITA {
        (String::new(), tax_code)
    } else {
        (tax_code, String::new())
    };
    let confezioni = i64::from(product.units) * line.quantity;

    vec![
        cell(4, period.to_string()),                               // D Data mese
        cell(5, esercizio),                                        // E Numero esercizio vicinato
        cell(6, cmnr),                                             // F CMNR rivendita
        cell(7, customer.ordinal_number.to_string()),             // G Numero ordinale
        cell(8, customer.municipality_name.clone()),              // H Comune
        cell(9, customer.province_name.clone()),                  // I Provincia
        cell(10, customer.vat_number.clone().unwrap_or_default()), // J CF/P.IVA
        cell(11, product.description.clone()),                    // K Denominazione
        cell(12, product.code.clone()),                           // L Codice prodotto
        cell(13, product.capacity.unwrap_or(0).to_string()),      // M Capacità
        cell(14, product.nicotine.unwrap_or(0).to_string()),      // N Nicotina
        cell(15, confezioni.to_string()),                         // O Numero di confezioni (B)
        // P Quantità totale (Litri) = template formula
        cell(17, invoice.number.to_string()),                     // Q N. Fattura (working)
        // R Accisa = template formula O*(M*0.124672) — left untouched so Excel computes it.
    ]
}

fn build_pat_row(
    customer: &Customer,
    product: &Product,
    invoice: &Invoice,
    line: &InvoiceLine,
    period: &str,
) -> Vec<TemplateCell> {
    let cmnr = if customer.typology == RIVENDITA {
        customer.tax_code.to_string()
    } else {
        String::new()
    };
    let confezioni = i64::from(product.packages.unwrap_or(0)) * line.quantity;

    vec![
        cell(5, period.to_string()),                               // E Data fine quindicina
        cell(6, cmnr),                                             // F CMNR rivendita
        cell(7, customer.ordinal_number.to_string()),             // G Numero ordinale
        cell(8, customer.municipality_name.clone()),              // H Comune
        cell(9, customer.province_name.clone()),                  // I Provincia
        cell(10, customer.vat_number.clone().unwrap_or_default()), // J CF/P.IVA
        cell(11, product.tabella.map(|t| t.to_string()).unwrap_or_default()), // K Tabella
        cell(12, product.adm_code.clone().unwrap_or_default()),   // L Codice prodotto (ADM)
        cell(13, product.description.clone()),                    // M Denominazione
        cell(14, product.units.to_string()),                      // N N° pezzi per confezione
        cell(15, confezioni.to_string()),                         // O N° confezioni immesse (B)
        // P N° totale pezzi (C=A*B) = template formula
        cell(17, product.code.clone()),                           // Q codice (working)
        cell(18, invoice.number.to_string()),                     // R N. Fattura (working)
        // S Accisa = Excel formula — left untouched so Excel computes it.
    ]
}
