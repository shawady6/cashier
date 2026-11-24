//! نظام نقاط البيع: نقطة الدخول.
//!
//! ملاحظة: وحدة الأساس اسمها `app_core` (وليس `core`) لتفادي الالتباس مع حزمة `core` في Rust.

pub mod app_core;
pub mod commands;
pub mod database;
pub mod modules;

use app_core::state::AppState;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // قاعدة البيانات في مجلد بيانات التطبيق، وليس في مجلد التثبيت.
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;

            let log_guard = app_core::logging::init(&data_dir.join("logs"))?;
            tracing::info!("بدء تشغيل التطبيق، مجلد البيانات: {}", data_dir.display());

            let pool = tauri::async_runtime::block_on(database::init(&data_dir.join("pos.db")))?;
            tracing::info!("قاعدة البيانات جاهزة");

            app.manage(AppState {
                db: pool,
                data_dir,
                log_guard,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::get_db_health,
            commands::get_settings,
            commands::set_setting,
            commands::list_business_profiles,
            commands::list_lookups,
            commands::save_lookup,
            commands::list_units,
            commands::create_unit,
            commands::list_products,
            commands::get_product,
            commands::save_product,
            commands::set_product_archived,
            commands::find_product_by_barcode,
            commands::set_product_image,
            commands::remove_product_image,
            commands::get_product_image,
        ])
        .run(tauri::generate_context!())
        .expect("خطأ أثناء تشغيل التطبيق");
}
