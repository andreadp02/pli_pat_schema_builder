use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::repository::settings as settings_repository;
use crate::AppError;

pub const KEY_ACCISA_PLI_PLN: &str = "accisa_pli_pln";
pub const KEY_ACCISA_PLI_PL: &str = "accisa_pli_pl";
pub const KEY_ACCISA_PAT: &str = "accisa_pat";

/// The three excise (accisa) coefficients used to build the tracciati accisa formulas: PLI with
/// nicotine (code `PLN…`), PLI without nicotine (code `PL…`), and PAT. Editable from the Template page.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccisaCoefficients {
    pub pli_pln: f64,
    pub pli_pl: f64,
    pub pat: f64,
}

pub async fn get_accisa_coefficients(db_path: &Path) -> Result<AccisaCoefficients, AppError> {
    settings_repository::get_accisa_coefficients(db_path.to_path_buf()).await
}

pub async fn save_accisa_coefficients(
    db_path: &Path,
    coefficients: AccisaCoefficients,
) -> Result<(), AppError> {
    validate(&coefficients)?;
    settings_repository::save_accisa_coefficients(db_path.to_path_buf(), coefficients).await
}

fn validate(coefficients: &AccisaCoefficients) -> Result<(), AppError> {
    for (name, value) in [
        ("PLI (con nicotina)", coefficients.pli_pln),
        ("PLI (senza nicotina)", coefficients.pli_pl),
        ("PAT", coefficients.pat),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(AppError::Processing(format!(
                "Coefficiente accisa {name} non valido: deve essere un numero positivo"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coeffs(pli_pln: f64, pli_pl: f64, pat: f64) -> AccisaCoefficients {
        AccisaCoefficients { pli_pln, pli_pl, pat }
    }

    #[test]
    fn accepts_positive_coefficients() {
        assert!(validate(&coeffs(0.124672, 0.124672, 0.0036)).is_ok());
    }

    #[test]
    fn rejects_non_positive_or_non_finite() {
        assert!(validate(&coeffs(0.0, 0.1, 0.1)).is_err()); // zero
        assert!(validate(&coeffs(-0.1, 0.1, 0.1)).is_err()); // negative
        assert!(validate(&coeffs(0.1, f64::NAN, 0.1)).is_err()); // NaN
        assert!(validate(&coeffs(0.1, 0.1, f64::INFINITY)).is_err()); // infinite
    }
}
