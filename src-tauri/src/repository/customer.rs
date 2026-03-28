use std::path::Path;
use std::path::PathBuf;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::AppError;

pub const TYPOLOGY_ESERCIZIO_DI_VICINATO: &str = "ESERCIZIO DI VICINATO";
pub const TYPOLOGY_RIVENDITA: &str = "RIVENDITA";
pub const TYPOLOGY_FARMACIA: &str = "FARMACIA";
pub const TYPOLOGY_PARAFARMACIA: &str = "PARAFARMACIA";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub id: i64,
    pub tax_code: i64,
    pub ordinal_number: i64,
    pub typology: String,
    pub vat_number: Option<String>,
    pub address: String,
    pub municipality_id: i64,
    pub municipality_name: String,
    pub province_name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewCustomer {
    pub tax_code: i64,
    pub ordinal_number: i64,
    pub typology: String,
    pub vat_number: Option<String>,
    pub address: String,
    pub municipality_name: String,
    pub province_name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCustomer {
    pub tax_code: Option<i64>,
    pub ordinal_number: Option<i64>,
    pub typology: Option<String>,
    pub vat_number: Option<Option<String>>,
    pub address: Option<String>,
    pub municipality_name: Option<String>,
    pub province_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedCustomers {
    pub items: Vec<Customer>,
    pub page: u32,
    pub page_size: u32,
    pub has_next_page: bool,
}

pub async fn create_customer(db_path: PathBuf, input: NewCustomer) -> Result<i64, AppError> {
    tauri::async_runtime::spawn_blocking(move || create_customer_sync(db_path.as_path(), input))
        .await
        .map_err(|e| AppError::Processing(format!("Create customer task failed: {e}")))?
}

pub async fn get_customers(
    db_path: PathBuf,
    page: u32,
    page_size: u32,
) -> Result<PaginatedCustomers, AppError> {
    tauri::async_runtime::spawn_blocking(move || get_customers_sync(db_path.as_path(), page, page_size))
        .await
        .map_err(|e| AppError::Processing(format!("Get customers task failed: {e}")))?
}

pub async fn get_customer_by_id(db_path: PathBuf, id: i64) -> Result<Option<Customer>, AppError> {
    tauri::async_runtime::spawn_blocking(move || get_customer_by_id_sync(db_path.as_path(), id))
        .await
        .map_err(|e| AppError::Processing(format!("Get customer task failed: {e}")))?
}

pub async fn update_customer(
    db_path: PathBuf,
    id: i64,
    input: UpdateCustomer,
) -> Result<bool, AppError> {
    tauri::async_runtime::spawn_blocking(move || update_customer_sync(db_path.as_path(), id, input))
        .await
        .map_err(|e| AppError::Processing(format!("Update customer task failed: {e}")))?
}

pub async fn delete_customer(db_path: PathBuf, id: i64) -> Result<bool, AppError> {
    tauri::async_runtime::spawn_blocking(move || delete_customer_sync(db_path.as_path(), id))
        .await
        .map_err(|e| AppError::Processing(format!("Delete customer task failed: {e}")))?
}

pub async fn create_customers_bulk(db_path: PathBuf, inputs: Vec<NewCustomer>) -> Result<usize, AppError> {
    tauri::async_runtime::spawn_blocking(move || create_customers_bulk_sync(db_path.as_path(), inputs))
        .await
        .map_err(|e| AppError::Processing(format!("Bulk create customer task failed: {e}")))?
}

fn open_connection(db_path: &Path) -> Result<Connection, AppError> {
    let conn = Connection::open(db_path).map_err(|e| AppError::Io(e.to_string()))?;
    conn.execute("PRAGMA foreign_keys = ON", [])
        .map_err(|e| AppError::Processing(e.to_string()))?;
    Ok(conn)
}

fn normalize_typology(value: &str) -> Result<String, AppError> {
    let normalized = value.trim().to_uppercase();
    match normalized.as_str() {
        TYPOLOGY_ESERCIZIO_DI_VICINATO
        | TYPOLOGY_RIVENDITA
        | TYPOLOGY_FARMACIA
        | TYPOLOGY_PARAFARMACIA => Ok(normalized),
        _ => Err(AppError::Processing(format!(
            "Invalid typology: '{value}'. Allowed values are ESERCIZIO DI VICINATO, RIVENDITA, FARMACIA, PARAFARMACIA"
        ))),
    }
}

fn normalize_required_field(value: &str, field_name: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::Processing(format!("{field_name} is required")));
    }
    Ok(trimmed.to_uppercase())
}

fn normalize_optional_field(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn upsert_municipality(conn: &Connection, municipality_name: &str, province_name: &str) -> Result<i64, AppError> {
    let normalized_municipality = normalize_required_field(municipality_name, "municipality name")?;
    let normalized_province = normalize_required_field(province_name, "province name")?;

    let existing_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM municipality WHERE name = ?1 AND province_name = ?2 LIMIT 1",
            params![normalized_municipality, normalized_province],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| AppError::Processing(e.to_string()))?;

    if let Some(id) = existing_id {
        return Ok(id);
    }

    conn.execute(
        "INSERT INTO municipality (name, province_name) VALUES (?1, ?2)",
        params![normalized_municipality, normalized_province],
    )
    .map_err(|e| AppError::Processing(e.to_string()))?;

    Ok(conn.last_insert_rowid())
}

fn create_customer_sync(db_path: &Path, input: NewCustomer) -> Result<i64, AppError> {
    let mut conn = open_connection(db_path)?;
    let tx = conn
        .transaction()
        .map_err(|e| AppError::Processing(e.to_string()))?;

    let typology = normalize_typology(&input.typology)?;
    let address = input.address.trim().to_string();
    if address.is_empty() {
        return Err(AppError::Processing("address is required".to_string()));
    }
    let vat_number = normalize_optional_field(input.vat_number);
    let municipality_id = upsert_municipality(&tx, &input.municipality_name, &input.province_name)?;

    tx.execute(
        "INSERT INTO customer (tax_code, ordinal_number, typology, vat_number, address, municipality_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            input.tax_code,
            input.ordinal_number,
            typology,
            vat_number,
            address,
            municipality_id
        ],
    )
    .map_err(|e| AppError::Processing(e.to_string()))?;

    let id = tx.last_insert_rowid();
    tx.commit().map_err(|e| AppError::Processing(e.to_string()))?;
    Ok(id)
}

fn get_customers_sync(db_path: &Path, page: u32, page_size: u32) -> Result<PaginatedCustomers, AppError> {
    let offset = (page.saturating_sub(1) as u64) * (page_size as u64);
    let conn = open_connection(db_path)?;

    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.tax_code, c.ordinal_number, c.typology, c.vat_number, c.address,
                    m.id, m.name, m.province_name
             FROM customer c
             JOIN municipality m ON m.id = c.municipality_id
             ORDER BY c.id DESC
             LIMIT ?1 OFFSET ?2",
        )
        .map_err(|e| AppError::Processing(e.to_string()))?;

    let mut rows = stmt
        .query_map(params![i64::from(page_size + 1), offset as i64], |row| {
            Ok(Customer {
                id: row.get(0)?,
                tax_code: row.get(1)?,
                ordinal_number: row.get(2)?,
                typology: row.get(3)?,
                vat_number: row.get(4)?,
                address: row.get(5)?,
                municipality_id: row.get(6)?,
                municipality_name: row.get(7)?,
                province_name: row.get(8)?,
            })
        })
        .map_err(|e| AppError::Processing(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::Processing(e.to_string()))?;

    let has_next_page = rows.len() > page_size as usize;
    if has_next_page {
        rows.truncate(page_size as usize);
    }

    Ok(PaginatedCustomers {
        items: rows,
        page,
        page_size,
        has_next_page,
    })
}

fn get_customer_by_id_sync(db_path: &Path, id: i64) -> Result<Option<Customer>, AppError> {
    let conn = open_connection(db_path)?;

    conn.query_row(
        "SELECT c.id, c.tax_code, c.ordinal_number, c.typology, c.vat_number, c.address,
                m.id, m.name, m.province_name
         FROM customer c
         JOIN municipality m ON m.id = c.municipality_id
         WHERE c.id = ?1
         LIMIT 1",
        params![id],
        |row| {
            Ok(Customer {
                id: row.get(0)?,
                tax_code: row.get(1)?,
                ordinal_number: row.get(2)?,
                typology: row.get(3)?,
                vat_number: row.get(4)?,
                address: row.get(5)?,
                municipality_id: row.get(6)?,
                municipality_name: row.get(7)?,
                province_name: row.get(8)?,
            })
        },
    )
    .optional()
    .map_err(|e| AppError::Processing(e.to_string()))
}

fn update_customer_sync(db_path: &Path, id: i64, input: UpdateCustomer) -> Result<bool, AppError> {
    let mut conn = open_connection(db_path)?;
    let tx = conn
        .transaction()
        .map_err(|e| AppError::Processing(e.to_string()))?;

    let existing = tx
        .query_row(
            "SELECT c.tax_code, c.ordinal_number, c.typology, c.vat_number, c.address,
                    m.name, m.province_name
             FROM customer c
             JOIN municipality m ON m.id = c.municipality_id
             WHERE c.id = ?1
             LIMIT 1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )
        .optional()
        .map_err(|e| AppError::Processing(e.to_string()))?;

    let Some(existing_row) = existing else {
        return Ok(false);
    };

    let tax_code = input.tax_code.unwrap_or(existing_row.0);
    let ordinal_number = input.ordinal_number.unwrap_or(existing_row.1);
    let typology = normalize_typology(input.typology.as_deref().unwrap_or(&existing_row.2))?;
    let vat_number = match input.vat_number {
        Some(value) => normalize_optional_field(value),
        None => existing_row.3,
    };
    let address = input
        .address
        .unwrap_or(existing_row.4)
        .trim()
        .to_string();
    if address.is_empty() {
        return Err(AppError::Processing("address is required".to_string()));
    }
    let municipality_name = input.municipality_name.unwrap_or(existing_row.5);
    let province_name = input.province_name.unwrap_or(existing_row.6);
    let municipality_id = upsert_municipality(&tx, &municipality_name, &province_name)?;

    let rows_affected = tx
        .execute(
            "UPDATE customer
             SET tax_code = ?1,
                 ordinal_number = ?2,
                 typology = ?3,
                 vat_number = ?4,
                 address = ?5,
                 municipality_id = ?6
             WHERE id = ?7",
            params![
                tax_code,
                ordinal_number,
                typology,
                vat_number,
                address,
                municipality_id,
                id
            ],
        )
        .map_err(|e| AppError::Processing(e.to_string()))?;

    tx.commit().map_err(|e| AppError::Processing(e.to_string()))?;
    Ok(rows_affected > 0)
}

fn delete_customer_sync(db_path: &Path, id: i64) -> Result<bool, AppError> {
    let conn = open_connection(db_path)?;
    let rows_affected = conn
        .execute("DELETE FROM customer WHERE id = ?1", params![id])
        .map_err(|e| AppError::Processing(e.to_string()))?;
    Ok(rows_affected > 0)
}

fn create_customers_bulk_sync(db_path: &Path, inputs: Vec<NewCustomer>) -> Result<usize, AppError> {
    if inputs.is_empty() {
        return Ok(0);
    }

    let mut conn = open_connection(db_path)?;
    let tx = conn
        .transaction()
        .map_err(|e| AppError::Processing(e.to_string()))?;

    for input in &inputs {
        let typology = normalize_typology(&input.typology)?;
        let address = input.address.trim().to_string();
        if address.is_empty() {
            return Err(AppError::Processing("address is required".to_string()));
        }
        let vat_number = normalize_optional_field(input.vat_number.clone());
        let municipality_id = upsert_municipality(&tx, &input.municipality_name, &input.province_name)?;

        tx.execute(
            "INSERT INTO customer (tax_code, ordinal_number, typology, vat_number, address, municipality_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(tax_code) DO UPDATE SET
                 ordinal_number = excluded.ordinal_number,
                 typology = excluded.typology,
                 vat_number = excluded.vat_number,
                 address = excluded.address,
                 municipality_id = excluded.municipality_id",
            params![
                input.tax_code,
                input.ordinal_number,
                typology,
                vat_number,
                address,
                municipality_id
            ],
        )
        .map_err(|e| AppError::Processing(e.to_string()))?;
    }

    tx.commit().map_err(|e| AppError::Processing(e.to_string()))?;
    Ok(inputs.len())
}
