//! أوامر الكتالوج: تصنيفات، ماركات، وحدات، منتجات. طبقة رفيعة فوق وحدات المنطق.

use tauri::State;

use crate::app_core::error::AppError;
use crate::app_core::state::AppState;
use crate::modules::lookups::{self, Kind, LookupInput, LookupItem};
use crate::modules::products::{
    self, images, BarcodeMatch, ProductDetail, ProductFilter, ProductInput, ProductListItem, Unit, UnitInput,
};

#[tauri::command]
pub async fn list_lookups(state: State<'_, AppState>, kind: Kind) -> Result<Vec<LookupItem>, AppError> {
    lookups::list(&state.db, kind).await
}

#[tauri::command]
pub async fn save_lookup(state: State<'_, AppState>, kind: Kind, input: LookupInput) -> Result<String, AppError> {
    lookups::save(&state.db, kind, input).await
}

#[tauri::command]
pub async fn list_units(state: State<'_, AppState>) -> Result<Vec<Unit>, AppError> {
    products::list_units(&state.db).await
}

#[tauri::command]
pub async fn create_unit(state: State<'_, AppState>, input: UnitInput) -> Result<String, AppError> {
    products::create_unit(&state.db, &input).await
}

#[tauri::command]
pub async fn list_products(
    state: State<'_, AppState>,
    filter: ProductFilter,
) -> Result<Vec<ProductListItem>, AppError> {
    products::list(&state.db, &filter).await
}

#[tauri::command]
pub async fn get_product(state: State<'_, AppState>, id: String) -> Result<ProductDetail, AppError> {
    products::get(&state.db, &id).await
}

#[tauri::command]
pub async fn save_product(state: State<'_, AppState>, input: ProductInput) -> Result<String, AppError> {
    products::save(&state.db, &input).await
}

#[tauri::command]
pub async fn set_product_archived(
    state: State<'_, AppState>,
    id: String,
    archived: bool,
) -> Result<(), AppError> {
    products::set_archived(&state.db, &id, archived).await
}

#[tauri::command]
pub async fn find_product_by_barcode(
    state: State<'_, AppState>,
    barcode: String,
) -> Result<Option<BarcodeMatch>, AppError> {
    products::find_by_barcode(&state.db, &barcode).await
}

#[tauri::command]
pub async fn set_product_image(
    state: State<'_, AppState>,
    id: String,
    data_base64: String,
) -> Result<(), AppError> {
    images::set_image(&state.db, &state.data_dir, &id, &data_base64).await
}

#[tauri::command]
pub async fn remove_product_image(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    images::remove_image(&state.db, &state.data_dir, &id).await
}

#[tauri::command]
pub async fn get_product_image(state: State<'_, AppState>, id: String) -> Result<Option<String>, AppError> {
    images::get_image(&state.db, &state.data_dir, &id).await
}
