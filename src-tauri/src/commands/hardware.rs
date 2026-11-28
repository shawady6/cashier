use tauri::State;

use crate::app_core::error::AppError;
use crate::app_core::state::AppState;
use crate::modules::hardware::{self, PrinterInfo, RasterInput};
use crate::modules::settings;

#[tauri::command]
pub async fn list_printers() -> Result<Vec<PrinterInfo>, AppError> {
    tauri::async_runtime::spawn_blocking(hardware::list_printers)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}

/// الطابعة تُقرأ من الإعدادات المحفوظة وليس من الواجهة. الواجهة ترسل صورة الإيصال فقط.
#[tauri::command]
pub async fn print_receipt_image(state: State<'_, AppState>, input: RasterInput, open_drawer: bool) -> Result<(), AppError> {
    let all = settings::get_all(&state.db).await?;
    tauri::async_runtime::spawn_blocking(move || hardware::print_image(&all, &input, open_drawer))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn open_cash_drawer(state: State<'_, AppState>) -> Result<(), AppError> {
    let all = settings::get_all(&state.db).await?;
    tauri::async_runtime::spawn_blocking(move || hardware::open_drawer(&all))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}
