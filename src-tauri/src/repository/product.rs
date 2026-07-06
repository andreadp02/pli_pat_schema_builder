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
    pub capacity: Option<f64>,
    pub nicotine: Option<f64>,
    pub packages: Option<u32>,
    /// ADM product code, written to tracciati_pat col L (PAT) or falls back to `code` in
    /// tracciati_pli. PAT sources it from the skeleton_pat; PLI derives it from `code` at import
    /// (see `service::product::pli_adm_code`).
    pub adm_code: Option<String>,
    /// "Tabella di commercializzazione" (PAT only), sourced from the skeleton_pat.
    pub tabella: Option<i64>,
}

impl Product {
    /// True once the skeleton has supplied the fields the tracciati need: PLI requires
    /// capacity+nicotine, PAT requires the ADM code. Incomplete products are skipped at generation.
    pub fn is_skeleton_complete(&self) -> bool {
        match self.product_type {
            ProductType::Pli => self.capacity.is_some() && self.nicotine.is_some(),
            ProductType::Pat => self.adm_code.as_deref().is_some_and(|c| !c.is_empty()),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewProduct {
    pub product_type: ProductType,
    pub code: String,
    pub description: String,
    pub units: u32,
    pub capacity: Option<f64>,
    pub nicotine: Option<f64>,
    pub packages: Option<u32>,
    #[serde(default)]
    pub adm_code: Option<String>,
    #[serde(default)]
    pub tabella: Option<i64>,
}

/// Skeleton-owned fields applied to an existing product matched by `code`.
#[derive(Debug, Clone)]
pub struct SkeletonUpdate {
    pub code: String,
    pub description: String,
    pub capacity: Option<f64>,
    pub nicotine: Option<f64>,
    pub adm_code: Option<String>,
    pub tabella: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProduct {
    pub code: Option<String>,
    pub description: Option<String>,
    pub units: Option<u32>,
    pub capacity: Option<f64>,
    pub nicotine: Option<f64>,
    pub packages: Option<u32>,
    #[serde(default)]
    pub adm_code: Option<String>,
    #[serde(default)]
    pub tabella: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedProducts {
    pub items: Vec<Product>,
    pub page: u32,
    pub page_size: u32,
    pub has_next_page: bool,
    pub total_count: u32,
}

const PRODUCT_COLUMNS: &str =
    "id, code, description, units, capacity, nicotine, packages, adm_code, tabella, product_type";

/// The type-specific columns split out: (capacity, nicotine, packages).
type TypeFields = (Option<f64>, Option<f64>, Option<u32>);

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
    incomplete_only: bool,
    code_search: Option<String>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
) -> Result<PaginatedProducts, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        get_products_sync(
            db_path.as_path(),
            page,
            page_size,
            product_type_filter,
            incomplete_only,
            code_search.as_deref(),
            sort_by.as_deref(),
            sort_dir.as_deref(),
        )
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

/// Applies skeleton-owned fields (description, and capacity/nicotine for PLI or adm_code for PAT)
/// to existing products matched by `code`. Returns how many rows were matched & updated.
pub async fn update_products_from_skeleton(
    db_path: PathBuf,
    rows: Vec<SkeletonUpdate>,
) -> Result<usize, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        update_products_from_skeleton_sync(db_path.as_path(), &rows)
    })
    .await
    .map_err(|e| AppError::Processing(format!("Skeleton update task failed: {e}")))?
}

fn normalize_product_code(code: &str) -> String {
    code.trim().to_uppercase()
}

/// adm_code is stored uppercased/trimmed like `code`; empty collapses to None so a blank skeleton
/// cell (via NULLIF) leaves the existing value untouched.
fn normalize_adm_code(adm_code: Option<&str>) -> Option<String> {
    adm_code
        .map(normalize_product_code)
        .filter(|code| !code.is_empty())
}

/// Validate the type-specific fields and return them split into (capacity, nicotine, packages),
/// enforcing the same per-type invariant as the table's CHECK constraint.
fn split_type_fields(
    product_type: ProductType,
    capacity: Option<f64>,
    nicotine: Option<f64>,
    packages: Option<u32>,
) -> Result<TypeFields, AppError> {
    match product_type {
        // PLI capacity/nicotine are skeleton-owned and may be NULL until enriched; only the shape
        // (no packages) is enforced here, matching the table CHECK.
        ProductType::Pli => Ok((capacity, nicotine, None)),
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

fn opt_f64(value: Option<f64>) -> Value {
    value.map_or(Value::Null, Value::from)
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
        adm_code,
        tabella,
    } = input;

    let normalized_code = normalize_product_code(&code);
    if normalized_code.is_empty() {
        return Err(AppError::Processing("Product code cannot be empty".to_string()));
    }

    let (capacity, nicotine, packages) =
        split_type_fields(product_type, capacity, nicotine, packages)?;

    conn.execute(
        "INSERT INTO product (product_type, code, description, units, capacity, nicotine, packages, adm_code, tabella)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            product_type.as_str(),
            normalized_code,
            description,
            units,
            opt_f64(capacity),
            opt_f64(nicotine),
            opt_value(packages),
            normalize_adm_code(adm_code.as_deref()),
            tabella
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
        let mut params_values: Vec<Value> = Vec::with_capacity(chunk.len() * 9);

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
            params_values.push(opt_f64(capacity));
            params_values.push(opt_f64(nicotine));
            params_values.push(opt_value(packages));
            params_values.push(match normalize_adm_code(product.adm_code.as_deref()) {
                Some(code) => Value::from(code),
                None => Value::Null,
            });
            params_values.push(match product.tabella {
                Some(tabella) => Value::from(tabella),
                None => Value::Null,
            });
        }

        total_affected += tx
            .execute(&sql, params_from_iter(params_values))
            .map_err(|e| AppError::Processing(e.to_string()))?;
    }

    Ok(total_affected)
}

fn update_products_from_skeleton_sync(
    db_path: &Path,
    rows: &[SkeletonUpdate],
) -> Result<usize, AppError> {
    if rows.is_empty() {
        return Ok(0);
    }

    let mut conn = Connection::open(db_path).map_err(|e| AppError::Io(e.to_string()))?;
    let tx = conn
        .transaction()
        .map_err(|e| AppError::Processing(e.to_string()))?;

    let mut matched = 0usize;
    {
        // COALESCE keeps the existing value when a field is NULL/empty, so one statement serves
        // both PLI (capacity+nicotine) and PAT (adm_code) skeleton rows without clobbering.
        let mut stmt = tx
            .prepare(
                "UPDATE product SET
                     description = COALESCE(NULLIF(?1, ''), description),
                     capacity = COALESCE(?2, capacity),
                     nicotine = COALESCE(?3, nicotine),
                     adm_code = COALESCE(NULLIF(?4, ''), adm_code),
                     tabella = COALESCE(?5, tabella)
                 WHERE code = ?6",
            )
            .map_err(|e| AppError::Processing(e.to_string()))?;

        for row in rows {
            let code = normalize_product_code(&row.code);
            if code.is_empty() {
                continue;
            }
            matched += stmt
                .execute(params![
                    row.description,
                    row.capacity,
                    row.nicotine,
                    normalize_adm_code(row.adm_code.as_deref()).unwrap_or_default(),
                    row.tabella,
                    code,
                ])
                .map_err(|e| AppError::Processing(e.to_string()))?;
        }
    }

    tx.commit().map_err(|e| AppError::Processing(e.to_string()))?;
    Ok(matched)
}

fn build_product_batch_insert_sql(row_count: usize) -> String {
    let values = std::iter::repeat_n("(?, ?, ?, ?, ?, ?, ?, ?, ?)", row_count)
        .collect::<Vec<_>>()
        .join(", ");
    // On re-import update only import-owned columns; description/capacity/nicotine/tabella are
    // owned by the skeleton phase and must survive a re-import. `IS NOT` is NULL-safe. adm_code is
    // import-owned for PLI (derived from `code`) but skeleton-owned for PAT (import always passes
    // NULL for PAT), so COALESCE(excluded.adm_code, product.adm_code) updates it for PLI while
    // leaving a PAT skeleton value untouched.
    // ponytail: a product code that flips type between imports can trip the table CHECK
    // (old capacity/nicotine kept against a new pat type) — accept the error; codes don't change type.
    format!(
        "INSERT INTO product (product_type, code, description, units, capacity, nicotine, packages, adm_code, tabella) VALUES {values}
         ON CONFLICT(code) DO UPDATE SET
             product_type = excluded.product_type,
             units = excluded.units,
             packages = excluded.packages,
             adm_code = COALESCE(excluded.adm_code, product.adm_code)
         WHERE product.product_type IS NOT excluded.product_type
            OR product.units IS NOT excluded.units
            OR product.packages IS NOT excluded.packages
            OR product.adm_code IS NOT COALESCE(excluded.adm_code, product.adm_code)"
    )
}

/// Maps a whitelisted frontend sort key to its real column; anything unknown falls back to `id`.
/// The result is always a `&'static str`, so splicing it into a query with `format!` is injection-safe.
fn sort_column(sort_by: Option<&str>) -> &'static str {
    match sort_by {
        Some("code") => "code",
        Some("description") => "description",
        Some("units") => "units",
        Some("productType") => "product_type",
        Some("admCode") => "adm_code",
        _ => "id",
    }
}

fn sort_direction(sort_dir: Option<&str>) -> &'static str {
    if sort_dir == Some("asc") {
        "ASC"
    } else {
        "DESC"
    }
}

fn get_products_sync(
    db_path: &Path,
    page: u32,
    page_size: u32,
    product_type_filter: Option<ProductType>,
    incomplete_only: bool,
    code_search: Option<&str>,
    sort_by: Option<&str>,
    sort_dir: Option<&str>,
) -> Result<PaginatedProducts, AppError> {
    let offset = (page.saturating_sub(1) as u64) * (page_size as u64);

    let (where_sql, where_params) =
        build_products_where(product_type_filter, incomplete_only, code_search);

    let conn = Connection::open(db_path).map_err(|e| AppError::Io(e.to_string()))?;

    let total_count: u32 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM product{where_sql}"),
            params_from_iter(where_params.clone()),
            |row| row.get(0),
        )
        .map_err(|e| AppError::Processing(e.to_string()))?;

    let sort_col = sort_column(sort_by);
    let dir = sort_direction(sort_dir);
    let query = format!(
        "SELECT {PRODUCT_COLUMNS} FROM product{where_sql} ORDER BY {sort_col} {dir}, id DESC LIMIT ? OFFSET ?"
    );
    let mut params_values = where_params;
    params_values.push(Value::from(i64::from(page_size + 1)));
    params_values.push(Value::from(offset as i64));

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
        total_count,
    })
}

/// Builds the shared `WHERE ...` clause (empty string if no filters) plus its bound params, reused
/// by both the COUNT query and the page query so the two never drift apart.
fn build_products_where(
    product_type_filter: Option<ProductType>,
    incomplete_only: bool,
    code_search: Option<&str>,
) -> (String, Vec<Value>) {
    let mut params_values: Vec<Value> = Vec::with_capacity(3);
    let mut conditions: Vec<&str> = Vec::new();

    if let Some(product_type) = product_type_filter {
        conditions.push("product_type = ?");
        params_values.push(Value::from(product_type.as_str().to_string()));
    }
    if let Some(term) = code_search.map(str::trim).filter(|t| !t.is_empty()) {
        // Case-insensitive substring match (SQLite LIKE is case-insensitive for ASCII).
        conditions.push("code LIKE ?");
        params_values.push(Value::from(format!("%{term}%")));
    }
    if incomplete_only {
        // Inverse of Product::is_skeleton_complete: PLI missing capacity/nicotine, PAT missing adm_code.
        conditions.push(
            "((product_type = 'pli' AND (capacity IS NULL OR nicotine IS NULL)) \
              OR (product_type = 'pat' AND (adm_code IS NULL OR adm_code = '')))",
        );
    }

    if conditions.is_empty() {
        (String::new(), params_values)
    } else {
        (format!(" WHERE {}", conditions.join(" AND ")), params_values)
    }
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
    let next_adm_code = input
        .adm_code
        .map(|code| normalize_product_code(&code))
        .or(existing.adm_code);
    let next_tabella = input.tabella.or(existing.tabella);

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
             SET code = ?1, description = ?2, units = ?3, capacity = ?4, nicotine = ?5, packages = ?6, adm_code = ?7, tabella = ?8
             WHERE id = ?9",
            params![
                next_code,
                next_description,
                next_units,
                opt_f64(next_capacity),
                opt_f64(next_nicotine),
                opt_value(next_packages),
                next_adm_code,
                next_tabella,
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
    let product_type_value: String = row.get(9)?;
    let product_type = match product_type_value.as_str() {
        "pli" => ProductType::Pli,
        "pat" => ProductType::Pat,
        other => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                9,
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
        adm_code: row.get(7)?,
        tabella: row.get(8)?,
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
                capacity REAL,
                nicotine REAL,
                packages INTEGER,
                adm_code TEXT,
                tabella INTEGER,
                CHECK (
                    (product_type = 'pli' AND packages IS NULL)
                 OR (product_type = 'pat' AND capacity IS NULL AND nicotine IS NULL)
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
        capacity: Option<f64>,
        nicotine: Option<f64>,
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
            adm_code: None,
            tabella: None,
        }
    }

    #[test]
    fn lookup_by_code_finds_each_type_and_respects_filter() {
        let db = temp_db();
        create_product_sync(&db, new(ProductType::Pli, "p1", Some(10.0), Some(5.0), None)).unwrap();
        create_product_sync(&db, new(ProductType::Pat, "a1", None, None, Some(3))).unwrap();

        let pli = get_product_by_code_sync(&db, "p1", None).unwrap().unwrap();
        assert_eq!(pli.product_type, ProductType::Pli);
        assert_eq!((pli.capacity, pli.nicotine, pli.packages), (Some(10.0), Some(5.0), None));

        let pat = get_product_by_code_sync(&db, "a1", None).unwrap().unwrap();
        assert_eq!(pat.product_type, ProductType::Pat);
        assert_eq!((pat.capacity, pat.nicotine, pat.packages), (None, None, Some(3)));

        // A type filter excludes the wrong table, and codes are normalized (trim + uppercase).
        assert!(get_product_by_code_sync(&db, "p1", Some(ProductType::Pat)).unwrap().is_none());
        assert!(get_product_by_code_sync(&db, "  p1 ", None).unwrap().is_some());

        std::fs::remove_file(&db).ok();
    }

    #[test]
    fn unenriched_products_insert_but_report_incomplete() {
        let db = temp_db();
        // Import leaves skeleton-owned fields NULL: a PLI without capacity/nicotine and a PAT
        // without adm_code both insert fine but are not yet usable for tracciati.
        create_product_sync(&db, new(ProductType::Pli, "p1", None, None, None)).unwrap();
        create_product_sync(&db, new(ProductType::Pat, "a1", None, None, Some(3))).unwrap();

        let pli = get_product_by_code_sync(&db, "p1", None).unwrap().unwrap();
        let pat = get_product_by_code_sync(&db, "a1", None).unwrap().unwrap();
        assert!(!pli.is_skeleton_complete());
        assert!(!pat.is_skeleton_complete());

        std::fs::remove_file(&db).ok();
    }

    #[test]
    fn skeleton_update_fills_only_owned_fields_and_reimport_preserves_them() {
        let db = temp_db();
        // Import leaves skeleton-owned fields NULL, as upload_products_excel now writes them.
        create_product_sync(&db, new(ProductType::Pli, "p1", None, None, None)).unwrap();
        create_product_sync(&db, new(ProductType::Pat, "a1", None, None, Some(2))).unwrap();

        // Both start incomplete until the skeleton enriches them.
        assert!(!get_product_by_code_sync(&db, "p1", None).unwrap().unwrap().is_skeleton_complete());
        assert!(!get_product_by_code_sync(&db, "a1", None).unwrap().unwrap().is_skeleton_complete());

        let matched = update_products_from_skeleton_sync(
            &db,
            &[
                SkeletonUpdate {
                    code: "p1".into(),
                    description: "MENTA".into(),
                    capacity: Some(20.0),
                    nicotine: Some(5.0),
                    adm_code: None,
                    tabella: None,
                },
                SkeletonUpdate {
                    code: "a1".into(),
                    description: "SMOKING KIT".into(),
                    capacity: None,
                    nicotine: None,
                    adm_code: Some("D00005012".into()),
                    tabella: Some(4),
                },
                SkeletonUpdate {
                    code: "missing".into(),
                    description: "x".into(),
                    capacity: None,
                    nicotine: None,
                    adm_code: None,
                    tabella: None,
                },
            ],
        )
        .unwrap();
        assert_eq!(matched, 2); // the unknown code is skipped

        let pli = get_product_by_code_sync(&db, "p1", None).unwrap().unwrap();
        assert_eq!((pli.description.as_str(), pli.capacity, pli.nicotine), ("MENTA", Some(20.0), Some(5.0)));
        assert!(pli.is_skeleton_complete());
        let pat = get_product_by_code_sync(&db, "a1", None).unwrap().unwrap();
        assert_eq!(
            (pat.description.as_str(), pat.adm_code.as_deref(), pat.tabella),
            ("SMOKING KIT", Some("D00005012"), Some(4))
        );
        assert!(pat.is_skeleton_complete());

        // A re-import (UPSERT) updates import-owned fields but must not wipe skeleton data —
        // including tabella, which is now skeleton-owned (import no longer sets it).
        create_products_in_batches_sync(
            &db,
            &[
                {
                    let mut p = new(ProductType::Pli, "p1", None, None, None);
                    p.units = 7;
                    p.description = String::new();
                    p
                },
                {
                    let mut p = new(ProductType::Pat, "a1", None, None, Some(9));
                    p.description = String::new();
                    p
                },
            ],
            10,
        )
        .unwrap();

        let pli = get_product_by_code_sync(&db, "p1", None).unwrap().unwrap();
        assert_eq!((pli.units, pli.description.as_str(), pli.capacity), (7, "MENTA", Some(20.0)));
        let pat = get_product_by_code_sync(&db, "a1", None).unwrap().unwrap();
        assert_eq!(
            (pat.units, pat.packages, pat.tabella, pat.adm_code.as_deref()),
            (1, Some(9), Some(4), Some("D00005012"))
        );

        std::fs::remove_file(&db).ok();
    }

    #[test]
    fn reimport_updates_pli_adm_code_but_never_clobbers_pat_skeleton_value() {
        let db = temp_db();
        {
            let mut p = new(ProductType::Pli, "p1", None, None, None);
            p.adm_code = Some("PL001".into());
            create_product_sync(&db, p).unwrap();
        }
        create_product_sync(&db, new(ProductType::Pat, "a1", None, None, Some(2))).unwrap();
        update_products_from_skeleton_sync(
            &db,
            &[SkeletonUpdate {
                code: "a1".into(),
                description: "d".into(),
                capacity: None,
                nicotine: None,
                adm_code: Some("D00005012".into()),
                tabella: None,
            }],
        )
        .unwrap();

        // Re-import: PLI's freshly-derived adm_code overwrites the old one; PAT keeps its
        // skeleton-set value since import always passes None for PAT.
        create_products_in_batches_sync(
            &db,
            &[
                {
                    let mut p = new(ProductType::Pli, "p1", None, None, None);
                    p.adm_code = Some("PL002".into());
                    p
                },
                new(ProductType::Pat, "a1", None, None, Some(2)),
            ],
            10,
        )
        .unwrap();

        let pli = get_product_by_code_sync(&db, "p1", None).unwrap().unwrap();
        assert_eq!(pli.adm_code.as_deref(), Some("PL002"));
        let pat = get_product_by_code_sync(&db, "a1", None).unwrap().unwrap();
        assert_eq!(pat.adm_code.as_deref(), Some("D00005012"));

        std::fs::remove_file(&db).ok();
    }

    #[test]
    fn update_product_sets_adm_code_and_preserves_it_when_absent() {
        let db = temp_db();
        let id =
            create_product_sync(&db, new(ProductType::Pli, "p1", Some(10.0), Some(5.0), None)).unwrap();

        update_product_sync(
            &db,
            id,
            UpdateProduct {
                code: None,
                description: None,
                units: None,
                capacity: None,
                nicotine: None,
                packages: None,
                adm_code: Some("D00009999".into()),
                tabella: None,
            },
        )
        .unwrap();
        let pli = get_product_by_code_sync(&db, "p1", None).unwrap().unwrap();
        assert_eq!(pli.adm_code.as_deref(), Some("D00009999"));

        // A subsequent update with adm_code = None must not clobber the value already set.
        update_product_sync(
            &db,
            id,
            UpdateProduct {
                code: None,
                description: None,
                units: Some(9),
                capacity: None,
                nicotine: None,
                packages: None,
                adm_code: None,
                tabella: None,
            },
        )
        .unwrap();
        let pli = get_product_by_code_sync(&db, "p1", None).unwrap().unwrap();
        assert_eq!((pli.units, pli.adm_code.as_deref()), (9, Some("D00009999")));

        std::fs::remove_file(&db).ok();
    }

    #[test]
    fn get_products_sorts_by_whitelisted_column_and_reports_total_count() {
        let db = temp_db();
        create_product_sync(&db, new(ProductType::Pli, "b", None, None, None)).unwrap();
        create_product_sync(&db, new(ProductType::Pli, "a", None, None, None)).unwrap();
        create_product_sync(&db, new(ProductType::Pli, "c", None, None, None)).unwrap();

        let asc = get_products_sync(&db, 1, 10, None, false, None, Some("code"), Some("asc")).unwrap();
        assert_eq!(
            asc.items.iter().map(|p| p.code.as_str()).collect::<Vec<_>>(),
            vec!["A", "B", "C"]
        );
        assert_eq!(asc.total_count, 3);

        // An unknown sort key must not be interpolated into the query; it falls back to `id DESC`.
        let unknown = get_products_sync(&db, 1, 10, None, false, None, Some("'; DROP TABLE product; --"), None).unwrap();
        assert_eq!(
            unknown.items.iter().map(|p| p.code.as_str()).collect::<Vec<_>>(),
            vec!["C", "A", "B"]
        );

        std::fs::remove_file(&db).ok();
    }
}
