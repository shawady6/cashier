//! أوامر Tauri: طبقة رفيعة تستدعي وحدات المنطق.

mod backup;
mod catalog;
mod finance;
mod hardware;
mod inventory;
mod reports;
mod sales;
pub use backup::*;
pub use catalog::*;
pub use finance::*;
pub use hardware::*;
pub use inventory::*;
pub use reports::*;
pub use sales::*;

use std::collections::HashMap;

use serde::Serialize;
use tauri::State;

use crate::app_core::error::AppError;
use crate::app_core::state::AppState;
use crate::modules::settings::{self, BusinessProfile};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub app_version: String,
    pub data_dir: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbHealth {
    pub sqlite_version: String,
    pub foreign_keys: bool,
    pub journal_mode: String,
    pub applied_migrations: i64,
}

#[tauri::command]
pub async fn get_app_info(state: State<'_, AppState>) -> Result<AppInfo, AppError> {
    Ok(AppInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        data_dir: state.data_dir.display().to_string(),
    })
}

#[tauri::command]
pub async fn get_db_health(state: State<'_, AppState>) -> Result<DbHealth, AppError> {
    let pool = &state.db;
    let sqlite_version: String = sqlx::query_scalar("SELECT sqlite_version()")
        .fetch_one(pool)
        .await?;
    let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(pool)
        .await?;
    let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(pool)
        .await?;
    let applied_migrations: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success = 1")
            .fetch_one(pool)
            .await?;
    Ok(DbHealth {
        sqlite_version,
        foreign_keys: foreign_keys == 1,
        journal_mode,
        applied_migrations,
    })
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<HashMap<String, String>, AppError> {
    settings::get_all(&state.db).await
}

#[tauri::command]
pub async fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), AppError> {
    settings::set(&state.db, &key, &value).await
}

#[tauri::command]
pub async fn list_business_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<BusinessProfile>, AppError> {
    settings::list_business_profiles(&state.db).await
}
