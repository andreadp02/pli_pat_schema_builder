use std::path::Path;
use std::path::PathBuf;

use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Row, Transaction};
use serde::{Deserialize, Serialize};

use crate::AppError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProductType {
    Pli,
    Pat,
}

impl ProductType {
    fn as_str(self) -> &'static str {
        match self {
            ProductType::Pli => "pli",
            ProductType::Pat => "pat",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub id: i64,
    pub code: String,
    pub description: String,
    pub units: u32,
    pub product_type: ProductType,
    pub capacity: Option<u32>,
    pub nicotine: Option<u32>,
    pub packages: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewProduct {
    pub product_type: ProductType,
    pub code: String,
    pub description: String,
    pub units: u32,
    pub capacity: Option<u32>,
    pub nicotine: Option<u32>,
    pub packages: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProduct {
    pub code: Option<String>,
    pub description: Option<String>,
    pub units: Option<u32>,
    pub capacity: Option<u32>,
    pub nicotine: Option<u32>,
    pub packages: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedProducts {
    pub items: Vec<Product>,
    pub page: u32,
    pub page_size: u32,
    pub has_next_page: bool,
}

const PRODUCT_COLUMNS: &str =
    "id, code, description, units, capacity, nicotine, packages, product_type";

/// The type-specific columns split out: (capacity, nicotine, packages).
type TypeFields = (Option<u32>, Option<u32>, Option<u32>);

pub async fn create_product(db_path: PathBuf, input: NewProduct) -> Result<i64, AppError> {
    tauri::async_runtime::spawn_blocking(move || create_product_sync(db_path.as_path(), input))
        .await
        .map_err(|e| AppError::Processing(format!("Create product task failed: {e}")))?
}

pub async fn get_products(
    db_path: PathBuf,
    page: u32,
    page_size: u32,
    product_type_filter: Option<ProductType>,
) -> Result<PaginatedProducts, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        get_products_sync(db_path.as_path(), page, page_size, product_type_filter)
    })
    .await
    .map_err(|e| AppError::Processing(format!("Get products task failed: {e}")))?
}

pub async fn get_product_by_code(
    db_path: PathBuf,
    code: String,
    product_type_filter: Option<ProductType>,
) -> Result<Option<Product>, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        get_product_by_code_sync(db_path.as_path(), &code, product_type_filter)
    })
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

/// Validate the type-specific fields and return them split into (capacity, nicotine, packages),
/// enforcing the same per-type invariant as the table's CHECK constraint.
fn split_type_fields(
    product_type: ProductType,
    capacity: Option<u32>,
    nicotine: Option<u32>,
    packages: Option<u32>,
) -> Result<TypeFields, AppError> {
    match product_type {
        ProductType::Pli => {
            let capacity = capacity.ok_or_else(|| {
                AppError::Processing("PLI product capacity is required".to_string())
            })?;
            let nicotine = nicotine.ok_or_else(|| {
                AppError::Processing("PLI product nicotine is required".to_string())
            })?;
            Ok((Some(capacity), Some(nicotine), None))
        }
        ProductType::Pat => {
            let packages = packages.ok_or_else(|| {
                AppError::Processing("PAT product packages is required".to_string())
            })?;
            Ok((None, None, Some(packages)))
        }
    }
}

fn opt_value(value: Option<u32>) -> Value {
    value.map_or(Value::Null, |v| Value::from(i64::from(v)))
}

fn create_product_sync(db_path: &Path, input: NewProduct) -> Result<i64, AppError> {
    let conn = Connection::open(db_path).map_err(|e| AppError::Io(e.to_string()))?;
    let NewProduct {
        product_type,
        code,
        description,
        units,
        capacity,
        nicotine,
        packages,
    } = input;

    let normalized_code = normalize_product_code(&code);
    if normalized_code.is_empty() {
        return Err(AppError::Processing("Product code cannot be empty".to_string()));
    }

    let (capacity, nicotine, packages) =
        split_type_fields(product_type, capacity, nicotine, packages)?;

    conn.execute(
        "INSERT INTO product (product_type, code, description, units, capacity, nicotine, packages)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            product_type.as_str(),
            normalized_code,
            description,
            units,
            opt_value(capacity),
            opt_value(nicotine),
            opt_value(packages)
        ],
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

    let total_affected = insert_products_in_batches(&tx, inputs, batch_size)?;

    tx.commit().map_err(|e| AppError::Processing(e.to_string()))?;
    Ok(total_affected)
}

fn insert_products_in_batches(
    tx: &Transaction<'_>,
    inputs: &[NewProduct],
    batch_size: usize,
) -> Result<usize, AppError> {
    let mut total_affected = 0;
    for chunk in inputs.chunks(batch_size) {
        let sql = build_product_batch_insert_sql(chunk.len());
        let mut params_values: Vec<Value> = Vec::with_capacity(chunk.len() * 7);

        for product in chunk {
            let normalized_code = normalize_product_code(&product.code);
            if normalized_code.is_empty() {
                return Err(AppError::Processing("Product code cannot be empty".to_string()));
            }
            let (capacity, nicotine, packages) = split_type_fields(
                product.product_type,
                product.capacity,
                product.nicotine,
                product.packages,
            )?;

            params_values.push(Value::from(product.product_type.as_str().to_string()));
            params_values.push(Value::from(normalized_code));
            params_values.push(Value::from(product.description.clone()));
            params_values.push(Value::from(i64::from(product.units)));
            params_values.push(opt_value(capacity));
            params_values.push(opt_value(nicotine));
            params_values.push(opt_value(packages));
        }

        total_affected += tx
            .execute(&sql, params_from_iter(params_values))
            .map_err(|e| AppError::Processing(e.to_string()))?;
    }

    Ok(total_affected)
}

fn build_product_batch_insert_sql(row_count: usize) -> String {
    let values = std::iter::repeat_n("(?, ?, ?, ?, ?, ?, ?)", row_count)
        .collect::<Vec<_>>()
        .join(", ");
    // `IS NOT` is NULL-safe, so the no-op guard works for the nullable type-specific columns.
    format!(
        "INSERT INTO product (product_type, code, description, units, capacity, nicotine, packages) VALUES {values}
         ON CONFLICT(code) DO UPDATE SET
             product_type = excluded.product_type,
             description = excluded.description,
             units = excluded.units,
             capacity = excluded.capacity,
             nicotine = excluded.nicotine,
             packages = excluded.packages
         WHERE product.product_type IS NOT excluded.product_type
            OR product.description IS NOT excluded.description
            OR product.units IS NOT excluded.units
            OR product.capacity IS NOT excluded.capacity
            OR product.nicotine IS NOT excluded.nicotine
            OR product.packages IS NOT excluded.packages"
    )
}

fn get_products_sync(
    db_path: &Path,
    page: u32,
    page_size: u32,
    product_type_filter: Option<ProductType>,
) -> Result<PaginatedProducts, AppError> {
    let offset = (page.saturating_sub(1) as u64) * (page_size as u64);

    let (query, params_values) = build_products_query(product_type_filter, page_size, offset);

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

fn build_products_query(
    product_type_filter: Option<ProductType>,
    page_size: u32,
    offset: u64,
) -> (String, Vec<Value>) {
    let mut params_values: Vec<Value> = Vec::with_capacity(3);

    let mut query = format!("SELECT {PRODUCT_COLUMNS} FROM product");
    if let Some(product_type) = product_type_filter {
        query.push_str(" WHERE product_type = ?");
        params_values.push(Value::from(product_type.as_str().to_string()));
    }
    query.push_str(" ORDER BY id DESC LIMIT ? OFFSET ?");
    params_values.push(Value::from(i64::from(page_size + 1)));
    params_values.push(Value::from(offset as i64));

    (query, params_values)
}

fn get_product_by_code_sync(
    db_path: &Path,
    code: &str,
    product_type_filter: Option<ProductType>,
) -> Result<Option<Product>, AppError> {
    let normalized_code = normalize_product_code(code);
    if normalized_code.is_empty() {
        return Ok(None);
    }

    let conn = Connection::open(db_path).map_err(|e| AppError::Io(e.to_string()))?;
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {PRODUCT_COLUMNS} FROM product
             WHERE code = ?1 AND (?2 IS NULL OR product_type = ?2)
             LIMIT 1"
        ))
        .map_err(|e| AppError::Processing(e.to_string()))?;

    let product = stmt
        .query_row(
            params![normalized_code, product_type_filter.map(ProductType::as_str)],
            map_product_row,
        )
        .optional()
        .map_err(|e| AppError::Processing(e.to_string()))?;

    Ok(product)
}

fn get_product_by_id_sync(db_path: &Path, id: i64) -> Result<Option<Product>, AppError> {
    let conn = Connection::open(db_path).map_err(|e| AppError::Io(e.to_string()))?;
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {PRODUCT_COLUMNS} FROM product WHERE id = ?1 LIMIT 1"
        ))
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
            .prepare(&format!(
                "SELECT {PRODUCT_COLUMNS} FROM product WHERE id = ?1 LIMIT 1"
            ))
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

    // product_type is immutable on edit; carry over each existing type-specific value as the default.
    let (next_capacity, next_nicotine, next_packages) = split_type_fields(
        existing.product_type,
        input.capacity.or(existing.capacity),
        input.nicotine.or(existing.nicotine),
        input.packages.or(existing.packages),
    )?;

    let rows_affected = conn
        .execute(
            "UPDATE product
             SET code = ?1, description = ?2, units = ?3, capacity = ?4, nicotine = ?5, packages = ?6
             WHERE id = ?7",
            params![
                next_code,
                next_description,
                next_units,
                opt_value(next_capacity),
                opt_value(next_nicotine),
                opt_value(next_packages),
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
    let product_type_value: String = row.get(7)?;
    let product_type = match product_type_value.as_str() {
        "pli" => ProductType::Pli,
        "pat" => ProductType::Pat,
        other => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                7,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Unknown product type: {other}"),
                )),
            ))
        }
    };

    Ok(Product {
        id: row.get(0)?,
        code: row.get(1)?,
        description: row.get(2)?,
        units: row.get(3)?,
        product_type,
        capacity: row.get(4)?,
        nicotine: row.get(5)?,
        packages: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("product_test_{nanos}.db"));
        let conn = Connection::open(&path).unwrap();
        // Mirrors the DDL in lib.rs::ensure_product_table_on_startup.
        conn.execute(
            "CREATE TABLE product (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                product_type TEXT NOT NULL CHECK(product_type IN ('pli','pat')),
                code TEXT NOT NULL CHECK(length(code) > 0) UNIQUE,
                description TEXT,
                units INTEGER NOT NULL,
                capacity INTEGER,
                nicotine INTEGER,
                packages INTEGER,
                CHECK (
                    (product_type = 'pli' AND capacity IS NOT NULL AND nicotine IS NOT NULL AND packages IS NULL)
                 OR (product_type = 'pat' AND packages IS NOT NULL AND capacity IS NULL AND nicotine IS NULL)
                )
            )",
            [],
        )
        .unwrap();
        path
    }

    fn new(
        product_type: ProductType,
        code: &str,
        capacity: Option<u32>,
        nicotine: Option<u32>,
        packages: Option<u32>,
    ) -> NewProduct {
        NewProduct {
            product_type,
            code: code.to_string(),
            description: "d".to_string(),
            units: 1,
            capacity,
            nicotine,
            packages,
        }
    }

    #[test]
    fn lookup_by_code_finds_each_type_and_respects_filter() {
        let db = temp_db();
        create_product_sync(&db, new(ProductType::Pli, "p1", Some(10), Some(5), None)).unwrap();
        create_product_sync(&db, new(ProductType::Pat, "a1", None, None, Some(3))).unwrap();

        let pli = get_product_by_code_sync(&db, "p1", None).unwrap().unwrap();
        assert_eq!(pli.product_type, ProductType::Pli);
        assert_eq!((pli.capacity, pli.nicotine, pli.packages), (Some(10), Some(5), None));

        let pat = get_product_by_code_sync(&db, "a1", None).unwrap().unwrap();
        assert_eq!(pat.product_type, ProductType::Pat);
        assert_eq!((pat.capacity, pat.nicotine, pat.packages), (None, None, Some(3)));

        // A type filter excludes the wrong table, and codes are normalized (trim + uppercase).
        assert!(get_product_by_code_sync(&db, "p1", Some(ProductType::Pat)).unwrap().is_none());
        assert!(get_product_by_code_sync(&db, "  p1 ", None).unwrap().is_some());

        std::fs::remove_file(&db).ok();
    }

    #[test]
    fn pli_without_required_fields_is_rejected() {
        let db = temp_db();
        assert!(create_product_sync(&db, new(ProductType::Pli, "p1", Some(10), None, None)).is_err());
        std::fs::remove_file(&db).ok();
    }
}
