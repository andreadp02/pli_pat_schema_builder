use std::path::Path;

use tauri::{command, AppHandle};

use crate::repository::product::{
    self, NewProduct, PaginatedProducts, Product, ProductType, UpdateProduct,
};
use crate::service;
use crate::utils::resolve_db_path;

const MAX_PRODUCTS_PAGE_SIZE: u32 = 1000;

#[command]
pub async fn create_product(app_handle: AppHandle, input: NewProduct) -> Result<i64, String> {
    let db_path = resolve_db_path(&app_handle)?;

    product::create_product(db_path, input)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_products(
    app_handle: AppHandle,
    page: u32,
    page_size: u32,
    product_type_filter: Option<ProductType>,
) -> Result<PaginatedProducts, String> {
    if page_size > MAX_PRODUCTS_PAGE_SIZE {
        return Err(format!("page_size cannot exceed {MAX_PRODUCTS_PAGE_SIZE}"));
    }
    let db_path = resolve_db_path(&app_handle)?;
    let normalized_page = page.max(1);
    let normalized_page_size = page_size.max(1);

    product::get_products(
        db_path,
        normalized_page,
        normalized_page_size,
        product_type_filter,
    )
        .await
    .map_err(|e| e.to_string())
}

#[command]
pub async fn get_product_by_code(
    app_handle: AppHandle,
    code: String,
    product_type_filter: Option<ProductType>,
) -> Result<Option<Product>, String> {
    let db_path = resolve_db_path(&app_handle)?;

    product::get_product_by_code(db_path, code, product_type_filter)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_product_by_id(
    app_handle: AppHandle,
    id: i64,
) -> Result<Option<Product>, String> {
    let db_path = resolve_db_path(&app_handle)?;

    product::get_product_by_id(db_path, id)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn update_product(
    app_handle: AppHandle,
    id: i64,
    input: UpdateProduct,
) -> Result<bool, String> {
    let db_path = resolve_db_path(&app_handle)?;

    product::update_product(db_path, id, input)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn delete_product(
    app_handle: AppHandle,
    id: i64,
) -> Result<bool, String> {
    let db_path = resolve_db_path(&app_handle)?;

    product::delete_product(db_path, id)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn upload_products_excel(app_handle: AppHandle, file_path: String) -> Result<String, String> {
    let db_path = resolve_db_path(&app_handle)?;

    service::product::upload_products_excel(Path::new(&file_path), db_path.as_path())
        .await
        .map_err(|e| e.to_string())
}
