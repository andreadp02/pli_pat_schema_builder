use std::path::Path;

use tauri::{command, AppHandle};

use crate::repository::customer::{self, NewCustomer, PaginatedCustomers, UpdateCustomer};
use crate::service;
use crate::service::customer::{ProvinceResolution, ValidateCustomersExcelResult};
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
    typology_filter: Option<String>,
) -> Result<PaginatedCustomers, String> {
    let db_path = resolve_db_path(&app_handle)?;
    customer::get_customers(db_path, page.max(1), page_size.max(1), typology_filter)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_customer_by_tax_code(
    app_handle: AppHandle,
    tax_code: i64,
) -> Result<Option<customer::Customer>, String> {
    let db_path = resolve_db_path(&app_handle)?;
    customer::get_customer_by_tax_code(db_path, tax_code)
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

#[command]
pub async fn validate_customers_excel(
    app_handle: AppHandle,
    file_path: String,
) -> Result<ValidateCustomersExcelResult, String> {
    let db_path = resolve_db_path(&app_handle)?;

    service::customer::validate_customers_excel(Path::new(&file_path), db_path.as_path())
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn confirm_customers_excel_upload(
    app_handle: AppHandle,
    file_path: String,
    resolutions: Vec<ProvinceResolution>,
) -> Result<String, String> {
    let db_path = resolve_db_path(&app_handle)?;

    service::customer::confirm_customers_excel_upload(
        Path::new(&file_path),
        db_path.as_path(),
        resolutions,
    )
    .await
    .map_err(|e| e.to_string())
}

