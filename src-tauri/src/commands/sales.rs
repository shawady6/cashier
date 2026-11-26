//! أوامر الكاشير والمبيعات والمخزون الأساسي.

use tauri::State;

use crate::app_core::error::AppError;
use crate::app_core::state::AppState;
use crate::modules::inventory::{self, AdjustInput};
use crate::modules::products::{self, Sellable};
use crate::modules::sales::{
    self, history, suspended, CartInput, CartQuote, CheckoutResult, ReturnInput, ReturnResult, SaleDetail,
    SaleFilter, SaleListItem, SuspendedSale,
};

#[tauri::command]
pub async fn search_sellables(state: State<'_, AppState>, query: String) -> Result<Vec<Sellable>, AppError> {
    products::search_sellables(&state.db, &query, 40).await
}

#[tauri::command]
pub async fn price_cart(state: State<'_, AppState>, cart: CartInput) -> Result<CartQuote, AppError> {
    sales::price_cart(&state.db, &cart).await
}

#[tauri::command]
pub async fn checkout(state: State<'_, AppState>, cart: CartInput) -> Result<CheckoutResult, AppError> {
    sales::checkout(&state.db, &cart).await
}

#[tauri::command]
pub async fn list_sales(state: State<'_, AppState>, filter: SaleFilter) -> Result<Vec<SaleListItem>, AppError> {
    history::list(&state.db, &filter).await
}

#[tauri::command]
pub async fn get_sale(state: State<'_, AppState>, id: String) -> Result<SaleDetail, AppError> {
    history::get(&state.db, &id).await
}

#[tauri::command]
pub async fn create_sales_return(state: State<'_, AppState>, input: ReturnInput) -> Result<ReturnResult, AppError> {
    sales::create_return(&state.db, &input).await
}

#[tauri::command]
pub async fn suspend_sale(state: State<'_, AppState>, label: String, payload: String) -> Result<String, AppError> {
    suspended::suspend(&state.db, &label, &payload).await
}

#[tauri::command]
pub async fn list_suspended_sales(state: State<'_, AppState>) -> Result<Vec<SuspendedSale>, AppError> {
    suspended::list(&state.db).await
}

#[tauri::command]
pub async fn resume_sale(state: State<'_, AppState>, id: String) -> Result<SuspendedSale, AppError> {
    suspended::resume(&state.db, &id).await
}

#[tauri::command]
pub async fn discard_suspended_sale(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    suspended::discard(&state.db, &id).await
}

#[tauri::command]
pub async fn adjust_stock_many(state: State<'_, AppState>, inputs: Vec<AdjustInput>) -> Result<(), AppError> {
    inventory::adjust_many(&state.db, &inputs).await
}

#[tauri::command]
pub async fn adjust_stock(state: State<'_, AppState>, input: AdjustInput) -> Result<(), AppError> {
    inventory::adjust(&state.db, &input).await
}
