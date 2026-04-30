use std::path::Path;
use std::path::PathBuf;

use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: i64,
    pub code: String,
    pub description: String,
    pub units: u32,
    pub pli: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewProduct {
    pub code: String,
    pub description: String,
    pub units: u32,
    pub pli: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProduct {
    pub code: Option<String>,
    pub description: Option<String>,
    pub units: Option<u32>,
    pub pli: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedProducts {
    pub items: Vec<Product>,
    pub page: u32,
    pub page_size: u32,
    pub has_next_page: bool,
}

pub async fn create_product(db_path: PathBuf, input: NewProduct) -> Result<i64, AppError> {
    tauri::async_runtime::spawn_blocking(move || create_product_sync(db_path.as_path(), input))
        .await
        .map_err(|e| AppError::Processing(format!("Create product task failed: {e}")))?
}

pub async fn get_products(
    db_path: PathBuf,
    page: u32,
    page_size: u32,
    pli_filter: Option<bool>,
) -> Result<PaginatedProducts, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        get_products_sync(db_path.as_path(), page, page_size, pli_filter)
    })
    .await
    .map_err(|e| AppError::Processing(format!("Get products task failed: {e}")))?
}

pub async fn get_product_by_code(
    db_path: PathBuf,
    code: String,
) -> Result<Option<Product>, AppError> {
    tauri::async_runtime::spawn_blocking(move || get_product_by_code_sync(db_path.as_path(), &code))
        .await
        .map_err(|e| AppError::Processing(format!("Get product by code task failed: {e}")))?
}

pub async fn get_product_by_id(db_path: PathBuf, id: i64) -> Result<Option<Product>, AppError> {
    tauri::async_runtime::spawn_blocking(move || get_product_by_id_sync(db_path.as_path(), id))
        .await
        .map_err(|e| AppError::Processing(format!("Get product task failed: {e}")))?
}

pub async fn update_product(
    db_path: PathBuf,
    id: i64,
    input: UpdateProduct,
) -> Result<bool, AppError> {
    tauri::async_runtime::spawn_blocking(move || update_product_sync(db_path.as_path(), id, input))
        .await
        .map_err(|e| AppError::Processing(format!("Update product task failed: {e}")))?
}

pub async fn delete_product(db_path: PathBuf, id: i64) -> Result<bool, AppError> {
    tauri::async_runtime::spawn_blocking(move || delete_product_sync(db_path.as_path(), id))
        .await
        .map_err(|e| AppError::Processing(format!("Delete product task failed: {e}")))?
}

pub async fn create_products_in_batches(
    db_path: PathBuf,
    inputs: Vec<NewProduct>,
    batch_size: usize,
) -> Result<usize, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        create_products_in_batches_sync(db_path.as_path(), &inputs, batch_size)
    })
    .await
    .map_err(|e| AppError::Processing(format!("Batch create product task failed: {e}")))?
}

fn normalize_product_code(code: &str) -> String {
    code.trim().to_uppercase()
}

fn create_product_sync(db_path: &Path, input: NewProduct) -> Result<i64, AppError> {
    let conn = Connection::open(db_path).map_err(|e| AppError::Io(e.to_string()))?;
    let normalized_code = normalize_product_code(&input.code);
    if normalized_code.is_empty() {
        return Err(AppError::Processing("Product code cannot be empty".to_string()));
    }

    conn.execute(
        "INSERT INTO product (code, description, units, pli) VALUES (?1, ?2, ?3, ?4)",
        params![normalized_code, input.description, input.units, if input.pli { 1 } else { 0 }],
    )
    .map_err(|e| AppError::Processing(e.to_string()))?;

    Ok(conn.last_insert_rowid())
}

fn create_products_in_batches_sync(
    db_path: &Path,
    inputs: &[NewProduct],
    batch_size: usize,
) -> Result<usize, AppError> {
    if inputs.is_empty() {
        return Ok(0);
    }

    if batch_size == 0 {
        return Err(AppError::Processing("batch_size must be greater than 0".to_string()));
    }

    let mut conn = Connection::open(db_path).map_err(|e| AppError::Io(e.to_string()))?;
    let tx = conn
        .transaction()
        .map_err(|e| AppError::Processing(e.to_string()))?;

    let mut total_affected = 0;

    for chunk in inputs.chunks(batch_size) {
        let sql = build_batch_insert_sql(chunk.len());
        let mut params_values: Vec<Value> = Vec::with_capacity(chunk.len() * 4);

        for product in chunk {
            let normalized_code = normalize_product_code(&product.code);
            if normalized_code.is_empty() {
                return Err(AppError::Processing("Product code cannot be empty".to_string()));
            }
            params_values.push(Value::from(normalized_code));
            params_values.push(Value::from(product.description.clone()));
            params_values.push(Value::from(i64::from(product.units)));
            params_values.push(Value::from(if product.pli { 1_i64 } else { 0_i64 }));
        }

        total_affected += tx.execute(&sql, params_from_iter(params_values))
            .map_err(|e| AppError::Processing(e.to_string()))?;
    }

    tx.commit().map_err(|e| AppError::Processing(e.to_string()))?;
    Ok(total_affected)
}

fn build_batch_insert_sql(row_count: usize) -> String {
    let values = std::iter::repeat_n("(?, ?, ?, ?)", row_count)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "INSERT INTO product (code, description, units, pli) VALUES {values}
         ON CONFLICT(code) DO UPDATE SET
             description = excluded.description,
             units = excluded.units,
             pli = excluded.pli
         WHERE product.description != excluded.description 
            OR product.units != excluded.units 
            OR product.pli != excluded.pli"
    )
}

fn get_products_sync(
    db_path: &Path,
    page: u32,
    page_size: u32,
    pli_filter: Option<bool>,
) -> Result<PaginatedProducts, AppError> {
    let offset = (page.saturating_sub(1) as u64) * (page_size as u64);

    let mut query = String::from(
        "SELECT id, code, description, units, pli
         FROM product",
    );
    let mut conditions: Vec<&str> = Vec::new();
    let mut params_values: Vec<Value> = Vec::new();

    if let Some(pli) = pli_filter {
        conditions.push("pli = ?");
        params_values.push(Value::from(if pli { 1_i64 } else { 0_i64 }));
    }

    if !conditions.is_empty() {
        query.push_str(" WHERE ");
        query.push_str(&conditions.join(" AND "));
    }

    query.push_str(" ORDER BY id DESC LIMIT ? OFFSET ?");
    params_values.push(Value::from(i64::from(page_size + 1)));
    params_values.push(Value::from(offset as i64));

    let conn = Connection::open(db_path).map_err(|e| AppError::Io(e.to_string()))?;
    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| AppError::Processing(e.to_string()))?;

    let mut rows = stmt
        .query_map(params_from_iter(params_values), map_product_row)
        .map_err(|e| AppError::Processing(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::Processing(e.to_string()))?;

    let has_next_page = rows.len() > page_size as usize;
    if has_next_page {
        rows.truncate(page_size as usize);
    }

    Ok(PaginatedProducts {
        items: rows,
        page,
        page_size,
        has_next_page,
    })
}

fn get_product_by_code_sync(db_path: &Path, code: &str) -> Result<Option<Product>, AppError> {
    let normalized_code = normalize_product_code(code);

    let conn = Connection::open(db_path).map_err(|e| AppError::Io(e.to_string()))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, code, description, units, pli
             FROM product
             WHERE code = ?1
             LIMIT 1",
        )
        .map_err(|e| AppError::Processing(e.to_string()))?;

    let product = stmt
        .query_row(params![normalized_code], map_product_row)
        .optional()
        .map_err(|e| AppError::Processing(e.to_string()))?;

    Ok(product)
}

fn get_product_by_id_sync(db_path: &Path, id: i64) -> Result<Option<Product>, AppError> {
    let conn = Connection::open(db_path).map_err(|e| AppError::Io(e.to_string()))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, code, description, units, pli
             FROM product
             WHERE id = ?1
             LIMIT 1",
        )
        .map_err(|e| AppError::Processing(e.to_string()))?;

    let product = stmt
        .query_row(params![id], map_product_row)
        .optional()
        .map_err(|e| AppError::Processing(e.to_string()))?;

    Ok(product)
}

fn update_product_sync(db_path: &Path, id: i64, input: UpdateProduct) -> Result<bool, AppError> {
    let conn = Connection::open(db_path).map_err(|e| AppError::Io(e.to_string()))?;

    let existing = {
        let mut stmt = conn
            .prepare(
                "SELECT id, code, description, units, pli
                 FROM product
                 WHERE id = ?1
                 LIMIT 1",
            )
            .map_err(|e| AppError::Processing(e.to_string()))?;

        stmt.query_row(params![id], map_product_row)
            .optional()
            .map_err(|e| AppError::Processing(e.to_string()))?
    };

    let Some(existing) = existing else {
        return Ok(false);
    };

    let next_code = input
        .code
        .as_deref()
        .map(normalize_product_code)
        .unwrap_or(existing.code);
    if next_code.is_empty() {
        return Err(AppError::Processing("Product code cannot be empty".to_string()));
    }
    let next_description = input.description.unwrap_or(existing.description);
    let next_units = input.units.unwrap_or(existing.units);
    let next_pli = input.pli.unwrap_or(existing.pli);

    let rows_affected = conn
        .execute(
            "UPDATE product
             SET code = ?1, description = ?2, units = ?3, pli = ?4
             WHERE id = ?5",
            params![
                next_code,
                next_description,
                next_units,
                if next_pli { 1 } else { 0 },
                id
            ],
        )
        .map_err(|e| AppError::Processing(e.to_string()))?;

    Ok(rows_affected > 0)
}

fn delete_product_sync(db_path: &Path, id: i64) -> Result<bool, AppError> {
    let conn = Connection::open(db_path).map_err(|e| AppError::Io(e.to_string()))?;
    let rows_affected = conn
        .execute("DELETE FROM product WHERE id = ?1", params![id])
        .map_err(|e| AppError::Processing(e.to_string()))?;

    Ok(rows_affected > 0)
}

fn map_product_row(row: &Row<'_>) -> rusqlite::Result<Product> {
    Ok(Product {
        id: row.get(0)?,
        code: row.get(1)?,
        description: row.get(2)?,
        units: row.get(3)?,
        pli: row.get::<_, i64>(4)? != 0,
    })
}
