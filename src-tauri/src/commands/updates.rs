use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::UpdaterExt;

use crate::app_core::error::AppError;
use crate::app_core::state::AppState;
use crate::modules::backup;
use crate::modules::settings;
use crate::modules::updates::{Channel, UpdateInfo, UpdateProgress};

async fn find_update(app: &AppHandle, state: &AppState) -> Result<Option<tauri_plugin_updater::Update>, AppError> {
    let all = settings::get_all(&state.db).await?;
    let url: tauri::Url = Channel::from_settings(&all)
        .endpoint()
        .parse()
        .map_err(|_| AppError::Internal("رابط التحديث غير صالح".into()))?;
    let updater = app
        .updater_builder()
        .endpoints(vec![url])
        .map_err(|e| AppError::Internal(format!("إعداد التحديث غير صالح: {e}")))?
        .build()
        .map_err(|e| AppError::Internal(format!("إعداد التحديث غير صالح: {e}")))?;
    updater
        .check()
        .await
        .map_err(|e| AppError::Internal(format!("تعذر فحص التحديثات، تأكد من الاتصال بالإنترنت: {e}")))
}

#[tauri::command]
pub async fn check_update(app: AppHandle, state: State<'_, AppState>) -> Result<UpdateInfo, AppError> {
    let current_version = app.package_info().version.to_string();
    Ok(match find_update(&app, &state).await? {
        Some(u) => UpdateInfo {
            current_version,
            available: true,
            version: Some(u.version.clone()),
            notes: u.body.clone(),
            date: u.date.map(|d| d.to_string()),
        },
        None => UpdateInfo { current_version, available: false, version: None, notes: None, date: None },
    })
}

/// نسخة احتياطية سليمة أولًا، ثم تنزيل وتثبيت التحديث الموقّع. فشل النسخة يوقف التحديث.
#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, AppState>) -> Result<(), AppError> {
    let update = find_update(&app, &state)
        .await?
        .ok_or_else(|| AppError::Validation("لا يوجد تحديث متاح".into()))?;
    let all = settings::get_all(&state.db).await?;
    let ext = crate::modules::files::effective_backup_dir(&all, crate::commands::files::documents(&app).as_deref(), &state.data_dir);
    backup::create_backup(&state.db, &state.data_dir, "pre-update", &ext).await?;
    backup::prune(&backup::backups_dir(&state.data_dir), "pre-update", 5);

    let mut downloaded: u64 = 0;
    let progress = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let _ = progress.emit("update-progress", UpdateProgress { downloaded, total });
            },
            || {},
        )
        .await
        .map_err(|e| AppError::Internal(format!("فشل التحديث، لم يتغير شيء في البرنامج: {e}")))?;
    Ok(())
}
