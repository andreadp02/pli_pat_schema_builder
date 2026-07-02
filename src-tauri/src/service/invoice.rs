use std::path::Path;

use crate::repository::excel as excel_repository;
use crate::service::excel::ExcelRow;
use crate::AppError;

/// One excise line of an invoice (only lines with a value in the `Accise` column are kept).
#[derive(Debug, Clone)]
pub struct InvoiceLine {
    pub code: String,
    pub quantity: i64,
}

#[derive(Debug, Clone)]
pub struct Invoice {
    pub number: i64,
    pub date: (i32, i32, i32), // (year, month, day) from "Data documento" (cell AP20)
    pub fiscal_code: String,
    pub vat: String,
    pub lines: Vec<InvoiceLine>,
}

const DOC_NUMBER_LABELS: [&str; 3] = ["nr. documento", "n. documento", "numero documento"];

pub async fn parse_invoice(path: &Path) -> Result<Invoice, AppError> {
    let rows = excel_repository::read_excel(path).await?;
    parse_invoice_rows(&rows)
}

fn parse_invoice_rows(rows: &[ExcelRow]) -> Result<Invoice, AppError> {
    let raw_number = value_right_of_label(rows, &DOC_NUMBER_LABELS).ok_or_else(|| {
        AppError::Processing("Invoice number ('Nr. documento') not found".to_string())
    })?;
    let number = raw_number.trim().parse::<i64>().map_err(|_| {
        AppError::Processing(format!(
            "Invoice number must be an integer, found '{raw_number}'"
        ))
    })?;

    let raw_date = value_right_of_label(rows, &["data documento"]).ok_or_else(|| {
        AppError::Processing("Invoice date ('Data documento') not found".to_string())
    })?;
    let date = parse_date_dmy(&raw_date).ok_or_else(|| {
        AppError::Processing(format!("Invoice date must be DD/MM/YYYY, found '{raw_date}'"))
    })?;

    let fiscal_code = value_right_of_label(rows, &["cod.fisc"]).unwrap_or_default();
    let vat = value_right_of_label(rows, &["p.iva"]).unwrap_or_default();

    let lines = parse_lines(rows)?;

    Ok(Invoice {
        number,
        date,
        fiscal_code,
        vat,
        lines,
    })
}

/// Parses the invoice date the real files store as the text "DD/MM/YYYY".
// ponytail: string form only — a rare serial-date cell fails here and drops the invoice with a warning.
fn parse_date_dmy(value: &str) -> Option<(i32, i32, i32)> {
    let mut parts = value.trim().split('/');
    let day = parts.next()?.trim().parse().ok()?;
    let month = parts.next()?.trim().parse().ok()?;
    let year = parts.next()?.trim().parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((year, month, day))
}

fn parse_lines(rows: &[ExcelRow]) -> Result<Vec<InvoiceLine>, AppError> {
    let Some(header_index) = rows.iter().position(|row| {
        let has_articolo = row.cells.iter().any(|c| normalize(c).contains("articolo"));
        let has_accise = row.cells.iter().any(|c| normalize(c).contains("accis"));
        has_articolo && has_accise
    }) else {
        return Err(AppError::Processing(
            "Invoice line header ('Articolo' / 'Accise') not found".to_string(),
        ));
    };

    let header = &rows[header_index];
    let articolo_idx = column_containing(header, "articolo")
        .ok_or_else(|| AppError::Processing("Invoice missing 'Articolo' column".to_string()))?;
    let qta_idx = column_with(header, &["q.t", "qta", "quantit"])
        .ok_or_else(|| AppError::Processing("Invoice missing 'Q.tà' column".to_string()))?;
    let accise_idx = column_containing(header, "accis")
        .ok_or_else(|| AppError::Processing("Invoice missing 'Accise' column".to_string()))?;

    let mut lines = Vec::new();
    for (offset, row) in rows.iter().enumerate().skip(header_index + 1) {
        if cell(row, accise_idx).is_none() {
            continue; // only lines with a value in the Accise column
        }
        let Some(code) = cell(row, articolo_idx) else {
            continue;
        };
        let raw_qty = cell(row, qta_idx).unwrap_or_default();
        let quantity = parse_quantity(raw_qty).ok_or_else(|| {
            AppError::Processing(format!(
                "Invalid quantity '{raw_qty}' for article '{code}' at row {}",
                offset + 1
            ))
        })?;

        lines.push(InvoiceLine {
            code: code.to_string(),
            quantity,
        });
    }

    Ok(lines)
}

/// Finds the first cell whose normalized text contains one of `labels`, then returns the first
/// non-empty cell to its right on the same row (e.g. "Nr. documento" → the AN value).
fn value_right_of_label(rows: &[ExcelRow], labels: &[&str]) -> Option<String> {
    for row in rows {
        for (col, raw) in row.cells.iter().enumerate() {
            let norm = normalize(raw);
            if labels.iter().any(|label| norm.contains(label)) {
                if let Some(value) = row
                    .cells
                    .iter()
                    .skip(col + 1)
                    .map(|c| c.trim())
                    .find(|c| !c.is_empty())
                {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

fn column_containing(header: &ExcelRow, needle: &str) -> Option<usize> {
    header.cells.iter().position(|c| normalize(c).contains(needle))
}

fn column_with(header: &ExcelRow, needles: &[&str]) -> Option<usize> {
    header
        .cells
        .iter()
        .position(|c| needles.iter().any(|n| normalize(c).contains(n)))
}

fn cell(row: &ExcelRow, index: usize) -> Option<&str> {
    row.cells
        .get(index)
        .map(|c| c.trim())
        .filter(|c| !c.is_empty())
}

fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

fn parse_quantity(value: &str) -> Option<i64> {
    if let Ok(parsed) = value.parse::<i64>() {
        return Some(parsed);
    }
    let parsed = value.replace(',', ".").parse::<f64>().ok()?;
    (parsed.fract() == 0.0 && parsed >= 0.0).then_some(parsed as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(cells: &[(usize, &str)]) -> ExcelRow {
        let max = cells.iter().map(|(i, _)| *i).max().map_or(0, |m| m + 1);
        let mut data = vec![String::new(); max];
        for (i, v) in cells {
            data[*i] = (*v).to_string();
        }
        ExcelRow { cells: data }
    }

    fn sample() -> Vec<ExcelRow> {
        vec![
            row(&[(20, "P.IVA:"), (23, "02910280805"), (31, "COD.FISC:"), (36, "CRSGLM78T08L063J")]),
            row(&[(33, "Nr. documento"), (39, "00784"), (40, "/D")]),
            row(&[(33, "Data documento"), (41, "15/04/2026")]),
            row(&[(1, "Articolo"), (9, "Descrizione"), (27, "Q.tà"), (37, "Accise")]),
            row(&[(1, ".Rif"), (27, "0")]), // no Accise → skipped
            row(&[(1, "PLN013970"), (27, "4"), (37, "13,81")]),
        ]
    }

    #[test]
    fn parses_number_identifiers_and_only_excise_lines() {
        let invoice = parse_invoice_rows(&sample()).unwrap();
        assert_eq!(invoice.number, 784); // leading zeros stripped
        assert_eq!(invoice.date, (2026, 4, 15));
        assert_eq!(invoice.fiscal_code, "CRSGLM78T08L063J");
        assert_eq!(invoice.vat, "02910280805");
        assert_eq!(invoice.lines.len(), 1);
        assert_eq!(invoice.lines[0].code, "PLN013970");
        assert_eq!(invoice.lines[0].quantity, 4);
    }

    #[test]
    fn non_integer_invoice_number_is_an_error() {
        let mut rows = sample();
        rows[1] = row(&[(33, "Nr. documento"), (39, "784/D")]);
        assert!(parse_invoice_rows(&rows).is_err());
    }

    #[test]
    fn parses_and_rejects_dates() {
        assert_eq!(parse_date_dmy("15/04/2026"), Some((2026, 4, 15)));
        assert_eq!(parse_date_dmy(" 1/6/2026 "), Some((2026, 6, 1)));
        assert_eq!(parse_date_dmy("Pag.1/1"), None);
        assert_eq!(parse_date_dmy("15/04/2026/x"), None);
    }
}
