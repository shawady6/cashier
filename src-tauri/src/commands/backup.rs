use tauri::{AppHandle, State};

use crate::app_core::error::AppError;
use crate::app_core::state::AppState;
use crate::modules::backup::{self, BackupCheck, BackupInfo, BackupResult, Location};
use crate::modules::settings;

async fn ext_dir(state: &AppState) -> String {
    settings::get_all(&state.db).await.ok().and_then(|m| m.get("backup_external_dir").cloned()).unwrap_or_default()
}

#[tauri::command]
pub async fn list_backups(state: State<'_, AppState>) -> Result<Vec<BackupInfo>, AppError> {
    let ext = ext_dir(&state).await;
    Ok(backup::list_backups(&state.data_dir, &ext))
}

#[tauri::command]
pub async fn create_backup(state: State<'_, AppState>) -> Result<BackupResult, AppError> {
    let all = settings::get_all(&state.db).await?;
    let ext = all.get("backup_external_dir").cloned().unwrap_or_default();
    backup::create_backup(&state.db, &state.data_dir, "manual", &ext).await
}

#[tauri::command]
pub async fn check_backup(state: State<'_, AppState>, location: Location, file_name: String) -> Result<BackupCheck, AppError> {
    let ext = ext_dir(&state).await;
    backup::check_backup(&state.data_dir, &ext, location, &file_name).await
}

#[tauri::command]
pub async fn delete_backup(state: State<'_, AppState>, location: Location, file_name: String) -> Result<(), AppError> {
    let ext = ext_dir(&state).await;
    backup::delete_backup(&state.data_dir, &ext, location, &file_name)
}

/// يجهّز الاستعادة ثم يعيد تشغيل البرنامج؛ الاستبدال الفعلي يتم عند التشغيل قبل فتح القاعدة.
#[tauri::command]
pub async fn restore_backup(app: AppHandle, state: State<'_, AppState>, location: Location, file_name: String) -> Result<BackupInfo, AppError> {
    let ext = ext_dir(&state).await;
    let safety = backup::stage_restore(&state.db, &state.data_dir, &ext, location, &file_name).await?;
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(1500));
        app.restart();
    });
    Ok(safety)
}
