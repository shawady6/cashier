use tauri::State;

use crate::app_core::error::AppError;
use crate::app_core::state::AppState;
use crate::modules::reports::{self, DebtsSummary, InventorySummary, ProductProfitRow, ProfitLoss, PurchasesReport, Range, SalesReport};

#[tauri::command]
pub async fn report_sales(state: State<'_, AppState>, range: Range) -> Result<SalesReport, AppError> {
    reports::sales_report(&state.db, &range).await
}

#[tauri::command]
pub async fn report_product_profit(state: State<'_, AppState>, range: Range, sort: String, limit: i64) -> Result<Vec<ProductProfitRow>, AppError> {
    reports::product_profit(&state.db, &range, &sort, limit).await
}

#[tauri::command]
pub async fn report_purchases(state: State<'_, AppState>, range: Range) -> Result<PurchasesReport, AppError> {
    reports::purchases_report(&state.db, &range).await
}

#[tauri::command]
pub async fn report_profit_loss(state: State<'_, AppState>, range: Range) -> Result<ProfitLoss, AppError> {
    reports::profit_loss(&state.db, &range).await
}

#[tauri::command]
pub async fn report_debts(state: State<'_, AppState>) -> Result<DebtsSummary, AppError> {
    reports::debts_summary(&state.db).await
}

#[tauri::command]
pub async fn report_inventory(state: State<'_, AppState>) -> Result<InventorySummary, AppError> {
    reports::inventory_summary(&state.db).await
}

/// يعيد نص CSV فقط؛ الواجهة تحفظه بنافذة حفظ المتصفح، فلا صلاحية ملفات إضافية.
#[tauri::command]
pub async fn report_export_csv(state: State<'_, AppState>, kind: String, range: Range) -> Result<String, AppError> {
    reports::csv::export_csv(&state.db, &kind, &range).await
}
