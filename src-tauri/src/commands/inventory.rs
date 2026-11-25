//! أوامر المخزون والمشتريات والموردين.

use tauri::State;

use crate::app_core::error::AppError;
use crate::app_core::state::AppState;
use crate::modules::inventory::counts::{self, CountDetail, CountInput, CountLineInput, CountRow};
use crate::modules::inventory::reports::{self, BalanceFilter, BalancesReport, LowStockRow, MovementFilter, MovementRow};
use crate::modules::inventory::transfers::{self, TransferInput, TransferRow};
use crate::modules::inventory::warehouses::{self, Warehouse, WarehouseInput};
use crate::modules::purchasing::{
    purchases, returns, suppliers, PurchaseDetail, PurchaseFilter, PurchaseInput, PurchaseReturnInput, PurchaseRow,
    Supplier, SupplierInput, SupplierPaymentInput,
};

#[tauri::command]
pub async fn list_warehouses(state: State<'_, AppState>) -> Result<Vec<Warehouse>, AppError> {
    warehouses::list(&state.db).await
}
#[tauri::command]
pub async fn save_warehouse(state: State<'_, AppState>, input: WarehouseInput) -> Result<String, AppError> {
    warehouses::save(&state.db, &input).await
}
#[tauri::command]
pub async fn stock_balances(state: State<'_, AppState>, filter: BalanceFilter) -> Result<BalancesReport, AppError> {
    reports::balances(&state.db, &filter).await
}
#[tauri::command]
pub async fn stock_movements(state: State<'_, AppState>, filter: MovementFilter) -> Result<Vec<MovementRow>, AppError> {
    reports::movements(&state.db, &filter).await
}
#[tauri::command]
pub async fn low_stock_report(state: State<'_, AppState>) -> Result<Vec<LowStockRow>, AppError> {
    reports::low_stock(&state.db).await
}
#[tauri::command]
pub async fn create_transfer(state: State<'_, AppState>, input: TransferInput) -> Result<String, AppError> {
    transfers::create(&state.db, &input).await
}
#[tauri::command]
pub async fn list_transfers(state: State<'_, AppState>) -> Result<Vec<TransferRow>, AppError> {
    transfers::list(&state.db).await
}
#[tauri::command]
pub async fn create_count(state: State<'_, AppState>, input: CountInput) -> Result<String, AppError> {
    counts::create(&state.db, &input).await
}
#[tauri::command]
pub async fn list_counts(state: State<'_, AppState>) -> Result<Vec<CountRow>, AppError> {
    counts::list(&state.db).await
}
#[tauri::command]
pub async fn get_count(state: State<'_, AppState>, id: String) -> Result<CountDetail, AppError> {
    counts::get(&state.db, &id).await
}
#[tauri::command]
pub async fn save_count_lines(state: State<'_, AppState>, id: String, lines: Vec<CountLineInput>) -> Result<(), AppError> {
    counts::save_lines(&state.db, &id, &lines).await
}
#[tauri::command]
pub async fn finalize_count(state: State<'_, AppState>, id: String) -> Result<i64, AppError> {
    counts::finalize(&state.db, &id).await
}
#[tauri::command]
pub async fn cancel_count(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    counts::cancel(&state.db, &id).await
}

#[tauri::command]
pub async fn list_suppliers(state: State<'_, AppState>) -> Result<Vec<Supplier>, AppError> {
    suppliers::list(&state.db).await
}
#[tauri::command]
pub async fn save_supplier(state: State<'_, AppState>, input: SupplierInput) -> Result<String, AppError> {
    suppliers::save(&state.db, &input).await
}
#[tauri::command]
pub async fn list_purchases(state: State<'_, AppState>, filter: PurchaseFilter) -> Result<Vec<PurchaseRow>, AppError> {
    purchases::list(&state.db, &filter).await
}
#[tauri::command]
pub async fn get_purchase(state: State<'_, AppState>, id: String) -> Result<PurchaseDetail, AppError> {
    purchases::get(&state.db, &id).await
}
#[tauri::command]
pub async fn save_purchase_draft(state: State<'_, AppState>, input: PurchaseInput) -> Result<String, AppError> {
    purchases::save_draft(&state.db, &input).await
}
#[tauri::command]
pub async fn receive_purchase(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    purchases::receive_purchase(&state.db, &id).await
}
#[tauri::command]
pub async fn cancel_purchase_draft(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    purchases::cancel_draft(&state.db, &id).await
}
#[tauri::command]
pub async fn add_supplier_payment(state: State<'_, AppState>, input: SupplierPaymentInput) -> Result<String, AppError> {
    purchases::add_payment(&state.db, &input).await
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseReturnResult {
    pub return_no: String,
    pub credit: i64,
}
#[tauri::command]
pub async fn create_purchase_return(
    state: State<'_, AppState>,
    input: PurchaseReturnInput,
) -> Result<PurchaseReturnResult, AppError> {
    let (return_no, credit) = returns::create(&state.db, &input).await?;
    Ok(PurchaseReturnResult { return_no, credit })
}
