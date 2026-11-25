//! أوامر المالية: ورديات الخزينة، المصروفات، العملاء.

use tauri::State;

use crate::app_core::error::AppError;
use crate::app_core::state::AppState;
use crate::modules::finance::customers::{self, Customer, CustomerInput, CustomerPaymentInput, Statement};
use crate::modules::finance::expenses::{self, ExpenseCategory, ExpenseCategoryInput, ExpenseFilter, ExpenseInput, ExpenseReport};
use crate::modules::finance::sessions::{self, CashEntryInput, SessionSummary};

#[tauri::command]
pub async fn open_cash_session(state: State<'_, AppState>, opening: String, note: Option<String>) -> Result<String, AppError> {
    sessions::open(&state.db, &opening, note.as_deref().unwrap_or("")).await
}
#[tauri::command]
pub async fn current_cash_session(state: State<'_, AppState>) -> Result<Option<SessionSummary>, AppError> {
    sessions::current(&state.db).await
}
#[tauri::command]
pub async fn close_cash_session(state: State<'_, AppState>, id: String, counted: String, note: Option<String>) -> Result<SessionSummary, AppError> {
    sessions::close(&state.db, &id, &counted, note.as_deref().unwrap_or("")).await
}
#[tauri::command]
pub async fn list_cash_sessions(state: State<'_, AppState>) -> Result<Vec<SessionSummary>, AppError> {
    sessions::list(&state.db).await
}
#[tauri::command]
pub async fn add_cash_entry(state: State<'_, AppState>, input: CashEntryInput) -> Result<String, AppError> {
    sessions::add_entry(&state.db, &input).await
}
#[tauri::command]
pub async fn list_expense_categories(state: State<'_, AppState>) -> Result<Vec<ExpenseCategory>, AppError> {
    expenses::list_categories(&state.db).await
}
#[tauri::command]
pub async fn save_expense_category(state: State<'_, AppState>, input: ExpenseCategoryInput) -> Result<String, AppError> {
    expenses::save_category(&state.db, &input).await
}
#[tauri::command]
pub async fn create_expense(state: State<'_, AppState>, input: ExpenseInput) -> Result<String, AppError> {
    expenses::create(&state.db, &input).await
}
#[tauri::command]
pub async fn void_expense(state: State<'_, AppState>, id: String, reason: String) -> Result<(), AppError> {
    expenses::void(&state.db, &id, &reason).await
}
#[tauri::command]
pub async fn list_expenses(state: State<'_, AppState>, filter: ExpenseFilter) -> Result<ExpenseReport, AppError> {
    expenses::list(&state.db, &filter).await
}
#[tauri::command]
pub async fn list_customers(state: State<'_, AppState>) -> Result<Vec<Customer>, AppError> {
    customers::list(&state.db).await
}
#[tauri::command]
pub async fn save_customer(state: State<'_, AppState>, input: CustomerInput) -> Result<String, AppError> {
    customers::save(&state.db, &input).await
}
#[tauri::command]
pub async fn add_customer_payment(state: State<'_, AppState>, input: CustomerPaymentInput) -> Result<String, AppError> {
    customers::take_payment(&state.db, &input).await
}
#[tauri::command]
pub async fn customer_statement(state: State<'_, AppState>, id: String) -> Result<Statement, AppError> {
    customers::statement(&state.db, &id).await
}
