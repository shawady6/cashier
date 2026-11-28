use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::app_core::error::AppError;
use crate::app_core::state::AppState;
use crate::modules::files::{self, AppFolder};
use crate::modules::reports::{self, Range};

pub fn documents(app: &AppHandle) -> Option<PathBuf> {
    app.path().document_dir().ok()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppFolders {
    pub root: String,
    pub backups: String,
    pub reports: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedFile {
    pub path: String,
    pub file_name: String,
}

fn text(p: PathBuf) -> String {
    p.to_string_lossy().to_string()
}

#[tauri::command]
pub async fn app_folders(app: AppHandle, state: State<'_, AppState>) -> Result<AppFolders, AppError> {
    let docs = documents(&app);
    let d = docs.as_deref();
    Ok(AppFolders {
        root: text(files::ensure(d, &state.data_dir, AppFolder::Root)?),
        backups: text(files::ensure(d, &state.data_dir, AppFolder::Backups)?),
        reports: text(files::ensure(d, &state.data_dir, AppFolder::Reports)?),
    })
}

/// نافذة اختيار مجلد من ويندوز. المسار المختار يُتحقق منه عند الاستخدام في النسخ الاحتياطي.
#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Result<Option<String>, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog().file().blocking_pick_folder().and_then(|p| p.into_path().ok()).map(text)
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))
}

/// يفتح أحد مجلدات البرنامج المعروفة فقط في مستكشف ويندوز، وليس أي مسار.
#[tauri::command]
pub async fn open_app_folder(app: AppHandle, state: State<'_, AppState>, kind: String) -> Result<(), AppError> {
    let folder = AppFolder::parse(&kind).ok_or_else(|| AppError::Validation("مجلد غير معروف".into()))?;
    let path = files::ensure(documents(&app).as_deref(), &state.data_dir, folder)?;
    open_in_explorer(&path)
}

#[cfg(windows)]
fn open_in_explorer(path: &std::path::Path) -> Result<(), AppError> {
    std::process::Command::new("explorer").arg(path).spawn().map_err(|e| AppError::Internal(format!("تعذر فتح المجلد: {e}")))?;
    Ok(())
}

#[cfg(not(windows))]
fn open_in_explorer(_path: &std::path::Path) -> Result<(), AppError> {
    Err(AppError::Validation("فتح المجلد متاح على ويندوز فقط".into()))
}

/// يحفظ تقرير CSV (UTF-8 مع BOM ليفتح بالعربية في Excel) داخل مجلد Reports الثابت.
#[tauri::command]
pub async fn save_report_csv(app: AppHandle, state: State<'_, AppState>, kind: String, range: Range) -> Result<SavedFile, AppError> {
    let body = reports::csv::export_csv(&state.db, &kind, &range).await?;
    let dir = files::ensure(documents(&app).as_deref(), &state.data_dir, AppFolder::Reports)?;
    let base = format!("{}_{}_{}", kind, range.date_from.as_deref().unwrap_or("all"), range.date_to.as_deref().unwrap_or("all"));
    let path = files::unique_path(&dir, &base, "csv");
    std::fs::write(&path, format!("\u{feff}{body}"))?;
    let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    Ok(SavedFile { path: text(path), file_name })
}
