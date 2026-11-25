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
            commands::search_sellables,
            commands::price_cart,
            commands::checkout,
            commands::list_sales,
            commands::get_sale,
            commands::create_sales_return,
            commands::suspend_sale,
            commands::list_suspended_sales,
            commands::resume_sale,
            commands::discard_suspended_sale,
            commands::adjust_stock,
            commands::list_warehouses,
            commands::save_warehouse,
            commands::stock_balances,
            commands::stock_movements,
            commands::low_stock_report,
            commands::create_transfer,
            commands::list_transfers,
            commands::create_count,
            commands::list_counts,
            commands::get_count,
            commands::save_count_lines,
            commands::finalize_count,
            commands::cancel_count,
            commands::list_suppliers,
            commands::save_supplier,
            commands::list_purchases,
            commands::get_purchase,
            commands::save_purchase_draft,
            commands::receive_purchase,
            commands::cancel_purchase_draft,
            commands::add_supplier_payment,
            commands::create_purchase_return,
            commands::open_cash_session,
            commands::current_cash_session,
            commands::close_cash_session,
            commands::list_cash_sessions,
            commands::add_cash_entry,
            commands::list_expense_categories,
            commands::save_expense_category,
            commands::create_expense,
            commands::void_expense,
            commands::list_expenses,
            commands::list_customers,
            commands::save_customer,
            commands::add_customer_payment,
            commands::customer_statement,
        ])
        .run(tauri::generate_context!())
        .expect("خطأ أثناء تشغيل التطبيق");
}
