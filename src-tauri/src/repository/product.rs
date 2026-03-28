use std::path::Path;
use std::path::PathBuf;

use rusqlite::{params, Connection, OptionalExtension, Row};
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
) -> Result<PaginatedProducts, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        get_products_sync(db_path.as_path(), page, page_size)
    })
    .await
    .map_err(|e| AppError::Processing(format!("Get products task failed: {e}")))?
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

fn create_product_sync(db_path: &Path, input: NewProduct) -> Result<i64, AppError> {
    let conn = Connection::open(db_path).map_err(|e| AppError::Io(e.to_string()))?;

    conn.execute(
        "INSERT INTO product (code, description, units, pli) VALUES (?1, ?2, ?3, ?4)",
        params![input.code, input.description, input.units, if input.pli { 1 } else { 0 }],
    )
    .map_err(|e| AppError::Processing(e.to_string()))?;

    Ok(conn.last_insert_rowid())
}

fn get_products_sync(db_path: &Path, page: u32, page_size: u32) -> Result<PaginatedProducts, AppError> {
    let offset = (page.saturating_sub(1) as u64) * (page_size as u64);

    let conn = Connection::open(db_path).map_err(|e| AppError::Io(e.to_string()))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, code, description, units, pli
             FROM product
             ORDER BY id DESC
             LIMIT ?1 OFFSET ?2",
        )
        .map_err(|e| AppError::Processing(e.to_string()))?;

    let mut rows = stmt
        .query_map(params![i64::from(page_size + 1), offset as i64], map_product_row)
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

    let next_code = input.code.unwrap_or(existing.code);
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
