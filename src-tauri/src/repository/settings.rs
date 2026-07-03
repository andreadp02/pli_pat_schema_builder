use std::path::Path;
use std::path::PathBuf;

use rusqlite::{params, Connection};

use crate::service::settings::{
    AccisaCoefficients, KEY_ACCISA_PAT, KEY_ACCISA_PLI_PL, KEY_ACCISA_PLI_PLN,
};
use crate::AppError;

pub async fn get_accisa_coefficients(db_path: PathBuf) -> Result<AccisaCoefficients, AppError> {
    tauri::async_runtime::spawn_blocking(move || get_accisa_coefficients_sync(db_path.as_path()))
        .await
        .map_err(|e| AppError::Processing(format!("Get accisa coefficients task failed: {e}")))?
}

pub async fn save_accisa_coefficients(
    db_path: PathBuf,
    coefficients: AccisaCoefficients,
) -> Result<(), AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        save_accisa_coefficients_sync(db_path.as_path(), coefficients)
    })
    .await
    .map_err(|e| AppError::Processing(format!("Save accisa coefficients task failed: {e}")))?
}

fn open_connection(db_path: &Path) -> Result<Connection, AppError> {
    Connection::open(db_path).map_err(|e| AppError::Io(e.to_string()))
}

fn get_coefficient(conn: &Connection, key: &str) -> Result<f64, AppError> {
    let value: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .map_err(|e| AppError::Processing(format!("Missing setting '{key}': {e}")))?;

    value
        .parse::<f64>()
        .map_err(|_| AppError::Processing(format!("Setting '{key}' is not a number: '{value}'")))
}

fn get_accisa_coefficients_sync(db_path: &Path) -> Result<AccisaCoefficients, AppError> {
    let conn = open_connection(db_path)?;
    Ok(AccisaCoefficients {
        pli_pln: get_coefficient(&conn, KEY_ACCISA_PLI_PLN)?,
        pli_pl: get_coefficient(&conn, KEY_ACCISA_PLI_PL)?,
        pat: get_coefficient(&conn, KEY_ACCISA_PAT)?,
    })
}

fn save_accisa_coefficients_sync(
    db_path: &Path,
    coefficients: AccisaCoefficients,
) -> Result<(), AppError> {
    let conn = open_connection(db_path)?;
    let mut stmt = conn
        .prepare(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .map_err(|e| AppError::Processing(e.to_string()))?;

    for (key, value) in [
        (KEY_ACCISA_PLI_PLN, coefficients.pli_pln),
        (KEY_ACCISA_PLI_PL, coefficients.pli_pl),
        (KEY_ACCISA_PAT, coefficients.pat),
    ] {
        stmt.execute(params![key, value.to_string()])
            .map_err(|e| AppError::Processing(e.to_string()))?;
    }
    Ok(())
}
