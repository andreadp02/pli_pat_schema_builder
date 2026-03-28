use std::fs;
use std::path::{Path, PathBuf};

use tauri::{command, AppHandle, Manager};

use crate::repository::customer::{self, NewCustomer, PaginatedCustomers, UpdateCustomer};
use crate::service;

#[command]
pub async fn create_customer(app_handle: AppHandle, input: NewCustomer) -> Result<i64, String> {
    let db_path = products_db_path(&app_handle)?;
    customer::create_customer(db_path, input)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_customers(
    app_handle: AppHandle,
    page: u32,
    page_size: u32,
) -> Result<PaginatedCustomers, String> {
    let db_path = products_db_path(&app_handle)?;
    customer::get_customers(db_path, page.max(1), page_size.max(1))
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_customer_by_id(app_handle: AppHandle, id: i64) -> Result<Option<customer::Customer>, String> {
    let db_path = products_db_path(&app_handle)?;
    customer::get_customer_by_id(db_path, id)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn update_customer(
    app_handle: AppHandle,
    id: i64,
    input: UpdateCustomer,
) -> Result<bool, String> {
    let db_path = products_db_path(&app_handle)?;
    customer::update_customer(db_path, id, input)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn delete_customer(app_handle: AppHandle, id: i64) -> Result<bool, String> {
    let db_path = products_db_path(&app_handle)?;
    customer::delete_customer(db_path, id)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn upload_customers_excel(app_handle: AppHandle, file_path: String) -> Result<String, String> {
    let db_path = products_db_path(&app_handle)?;

    service::customer::upload_customers_excel(Path::new(&file_path), db_path.as_path())
        .await
        .map_err(|e| e.to_string())
}

fn products_db_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let mut db_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;

    fs::create_dir_all(&db_dir).map_err(|e| format!("Failed to create app data dir: {e}"))?;
    db_dir.push("products.db");
    Ok(db_dir)
}
