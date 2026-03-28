use std::path::Path;

use tauri::{command, AppHandle};

use crate::repository::customer::{self, NewCustomer, PaginatedCustomers, UpdateCustomer};
use crate::service;
use crate::utils::resolve_db_path;

#[command]
pub async fn create_customer(app_handle: AppHandle, input: NewCustomer) -> Result<i64, String> {
    let db_path = resolve_db_path(&app_handle)?;
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
    let db_path = resolve_db_path(&app_handle)?;
    customer::get_customers(db_path, page.max(1), page_size.max(1))
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_customer_by_id(app_handle: AppHandle, id: i64) -> Result<Option<customer::Customer>, String> {
    let db_path = resolve_db_path(&app_handle)?;
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
    let db_path = resolve_db_path(&app_handle)?;
    customer::update_customer(db_path, id, input)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn delete_customer(app_handle: AppHandle, id: i64) -> Result<bool, String> {
    let db_path = resolve_db_path(&app_handle)?;
    customer::delete_customer(db_path, id)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn upload_customers_excel(app_handle: AppHandle, file_path: String) -> Result<String, String> {
    let db_path = resolve_db_path(&app_handle)?;

    service::customer::upload_customers_excel(Path::new(&file_path), db_path.as_path())
        .await
        .map_err(|e| e.to_string())
}

