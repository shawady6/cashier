use tauri::{AppHandle, State};

use crate::commands::files::documents;
use crate::modules::files;

use crate::app_core::error::AppError;
use crate::app_core::state::AppState;
use crate::modules::backup::{self, BackupCheck, BackupInfo, BackupResult, Location};
use crate::modules::settings;

/// مجلد النسخ الإضافي: اختيار المستخدم أو مجلد البرنامج في المستندات.
async fn ext_dir(app: &AppHandle, state: &AppState) -> String {
    let all = settings::get_all(&state.db).await.unwrap_or_default();
    files::effective_backup_dir(&all, documents(app).as_deref(), &state.data_dir)
}

#[tauri::command]
pub async fn list_backups(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<BackupInfo>, AppError> {
    let ext = ext_dir(&app, &state).await;
    Ok(backup::list_backups(&state.data_dir, &ext))
}

#[tauri::command]
pub async fn create_backup(app: AppHandle, state: State<'_, AppState>) -> Result<BackupResult, AppError> {
    let ext = ext_dir(&app, &state).await;
    backup::create_backup(&state.db, &state.data_dir, "manual", &ext).await
}

#[tauri::command]
pub async fn check_backup(app: AppHandle, state: State<'_, AppState>, location: Location, file_name: String) -> Result<BackupCheck, AppError> {
    let ext = ext_dir(&app, &state).await;
    backup::check_backup(&state.data_dir, &ext, location, &file_name).await
}

#[tauri::command]
pub async fn delete_backup(app: AppHandle, state: State<'_, AppState>, location: Location, file_name: String) -> Result<(), AppError> {
    let ext = ext_dir(&app, &state).await;
    backup::delete_backup(&state.data_dir, &ext, location, &file_name)
}

/// يجهّز الاستعادة ثم يعيد تشغيل البرنامج؛ الاستبدال الفعلي يتم عند التشغيل قبل فتح القاعدة.
#[tauri::command]
pub async fn restore_backup(app: AppHandle, state: State<'_, AppState>, location: Location, file_name: String) -> Result<BackupInfo, AppError> {
    let ext = ext_dir(&app, &state).await;
    let safety = backup::stage_restore(&state.db, &state.data_dir, &ext, location, &file_name).await?;
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(1500));
        app.restart();
    });
    Ok(safety)
}
