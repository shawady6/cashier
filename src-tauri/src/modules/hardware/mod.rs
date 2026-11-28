//! الأجهزة المتصلة: طابعة الإيصالات (ESC/POS) ودُرج الكاش. الاسم يُؤخذ من الإعدادات المحفوظة
//! ويُتحقق أنه ضمن الطابعات المثبّتة، فلا تُرسل بيانات لجهاز لم يختره الفني.

pub mod escpos;
#[cfg(windows)]
mod win;

use std::collections::HashMap;

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::app_core::error::AppError;

pub const MAX_ROWS: usize = 6000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrinterInfo {
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RasterInput {
    pub width: usize,
    pub height: usize,
    /// بتات الصورة (1 = أسود)، كل صف مضاعفات 8 نقاط، مشفرة base64.
    pub data_base64: String,
}

#[cfg(windows)]
pub fn list_printers() -> Result<Vec<PrinterInfo>, AppError> {
    let default = win::default_printer();
    Ok(win::list_printers()?
        .into_iter()
        .map(|name| PrinterInfo { is_default: default.as_deref() == Some(name.as_str()), name })
        .collect())
}

#[cfg(not(windows))]
pub fn list_printers() -> Result<Vec<PrinterInfo>, AppError> {
    Ok(vec![])
}

#[cfg(windows)]
fn send(printer: &str, bytes: &[u8]) -> Result<(), AppError> {
    win::print_raw(printer, bytes)
}

#[cfg(not(windows))]
fn send(_printer: &str, _bytes: &[u8]) -> Result<(), AppError> {
    Err(AppError::Validation("الطباعة المباشرة متاحة على ويندوز فقط".into()))
}

/// عرض الصورة بالنقاط حسب عرض الورق (203 نقطة/بوصة تقريبًا).
pub fn dots_for_paper(width_mm: &str) -> usize {
    if width_mm == "58" {
        384
    } else {
        576
    }
}

pub fn decode_raster(input: &RasterInput, paper_width: &str) -> Result<Vec<u8>, AppError> {
    let bad = |m: &str| AppError::Validation(m.to_string());
    if input.width != dots_for_paper(paper_width) {
        return Err(bad("عرض الصورة لا يطابق عرض الورق المحدد"));
    }
    if input.height == 0 || input.height > MAX_ROWS {
        return Err(bad("ارتفاع الصورة غير صالح"));
    }
    let bits = base64::engine::general_purpose::STANDARD
        .decode(input.data_base64.as_bytes())
        .map_err(|_| bad("بيانات الصورة غير صالحة"))?;
    if bits.len() != input.width / 8 * input.height {
        return Err(bad("حجم بيانات الصورة لا يطابق أبعادها"));
    }
    Ok(bits)
}

/// يتحقق أن الطابعة المختارة مثبّتة فعلًا، ويعيد اسمها.
fn configured_printer(settings: &HashMap<String, String>) -> Result<String, AppError> {
    let name = settings.get("printer_name").map(|s| s.trim()).unwrap_or("");
    if name.is_empty() {
        return Err(AppError::Validation("لم يتم اختيار طابعة في الإعدادات المتقدمة".into()));
    }
    if !list_printers()?.iter().any(|p| p.name == name) {
        return Err(AppError::Validation("الطابعة المختارة غير موجودة أو غير مثبتة على الجهاز".into()));
    }
    Ok(name.to_string())
}

fn drawer_pin(settings: &HashMap<String, String>) -> u8 {
    if settings.get("drawer_pin").map(|s| s.as_str()) == Some("1") {
        1
    } else {
        0
    }
}

pub fn print_image(settings: &HashMap<String, String>, input: &RasterInput, open_drawer: bool) -> Result<(), AppError> {
    let paper = settings.get("receipt_paper_width").map(|s| s.as_str()).unwrap_or("80");
    let bits = decode_raster(input, paper)?;
    let printer = configured_printer(settings)?;
    let cut = settings.get("printer_cut").map(|s| s.as_str()) != Some("0");
    let copies: usize = settings.get("print_copies").and_then(|v| v.parse().ok()).filter(|n| (1..=3).contains(n)).unwrap_or(1);
    for i in 0..copies {
        let job = escpos::Job { image: Some((input.width, input.height, &bits)), cut, drawer_pin: if open_drawer && i == 0 { Some(drawer_pin(settings)) } else { None } };
        send(&printer, &escpos::build_job(&job))?;
    }
    Ok(())
}

pub fn open_drawer(settings: &HashMap<String, String>) -> Result<(), AppError> {
    let printer = configured_printer(settings)?;
    let job = escpos::Job { image: None, cut: false, drawer_pin: Some(drawer_pin(settings)) };
    send(&printer, &escpos::build_job(&job))
}

#[cfg(test)]
mod tests;
