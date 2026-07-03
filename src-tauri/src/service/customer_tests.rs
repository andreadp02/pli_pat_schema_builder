use super::*;

fn row(cells: &[&str]) -> ExcelRow {
    ExcelRow {
        cells: cells.iter().map(|c| c.to_string()).collect(),
    }
}

#[test]
fn trailing_row_without_tax_code_is_skipped_not_invalid() {
    let rows = vec![
        row(&["Numero esercizio vicinato / CMNR rivendita", "Num. ordinale punto vendita", "Tipologia punto vendita", "Comune punto vendita", "Indirizzo punto vendita"]),
        row(&["688033", "24", "RIVENDITA", "MELZO", "VIA VOLTA 100"]),
        // trailing junk row: stray non-whitespace value in an unmapped column, no tax_code
        row(&["", "", "", "", "", "0"]),
    ];

    let (parsed, invalid) = parse_customer_rows(&rows).unwrap();

    assert_eq!(parsed.len(), 1, "only the real data row should parse");
    assert!(invalid.is_empty(), "trailing junk row must be skipped, not reported invalid");
}

#[test]
fn out_of_enum_typology_is_reported_not_passed_to_insert() {
    let rows = vec![
        row(&["Numero esercizio vicinato / CMNR rivendita", "Num. ordinale punto vendita", "Tipologia punto vendita", "Comune punto vendita", "Indirizzo punto vendita"]),
        row(&["688033", "24", "TABACCHERIA", "MELZO", "VIA VOLTA 100"]),
    ];

    let (parsed, invalid) = parse_customer_rows(&rows).unwrap();

    assert!(parsed.is_empty(), "bad-typology row must not reach the batch insert");
    assert_eq!(invalid.len(), 1);
    assert!(invalid[0].message.contains("typology"));
}

#[test]
fn numeric_vat_leading_zeros_restored_to_11_digits() {
    // calamine reads a number-stored P.IVA as a float, dropping leading zeros
    assert_eq!(restore_vat_leading_zeros("3136090796"), "03136090796");
    assert_eq!(restore_vat_leading_zeros("3861020794"), "03861020794");
    // already 11 digits: untouched
    assert_eq!(restore_vat_leading_zeros("05384590872"), "05384590872");
    // alphanumeric codice fiscale: untouched
    assert_eq!(restore_vat_leading_zeros("FZZJCP95L05I537S"), "FZZJCP95L05I537S");
}
