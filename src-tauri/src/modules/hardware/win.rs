//! الطباعة الخام (RAW) عبر مُجدول الطباعة في ويندوز. تعريفات winspool مكتوبة يدويًا بدون اعتماديات.

use std::os::raw::c_void;

use crate::app_core::error::AppError;

type Bool = i32;

#[repr(C)]
struct PrinterInfo4W {
    printer_name: *mut u16,
    server_name: *mut u16,
    attributes: u32,
}

#[repr(C)]
struct DocInfo1W {
    doc_name: *mut u16,
    output_file: *mut u16,
    datatype: *mut u16,
}

#[link(name = "winspool")]
extern "system" {
    fn EnumPrintersW(flags: u32, name: *const u16, level: u32, buf: *mut u8, cb: u32, needed: *mut u32, returned: *mut u32) -> Bool;
    fn GetDefaultPrinterW(buf: *mut u16, size: *mut u32) -> Bool;
    fn OpenPrinterW(name: *const u16, handle: *mut *mut c_void, defaults: *const c_void) -> Bool;
    fn ClosePrinter(handle: *mut c_void) -> Bool;
    fn StartDocPrinterW(handle: *mut c_void, level: u32, info: *const DocInfo1W) -> u32;
    fn EndDocPrinter(handle: *mut c_void) -> Bool;
    fn StartPagePrinter(handle: *mut c_void) -> Bool;
    fn EndPagePrinter(handle: *mut c_void) -> Bool;
    fn WritePrinter(handle: *mut c_void, buf: *const c_void, cb: u32, written: *mut u32) -> Bool;
}

const PRINTER_ENUM_LOCAL: u32 = 0x2;
const PRINTER_ENUM_CONNECTIONS: u32 = 0x4;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn from_wide(p: *const u16) -> String {
    if p.is_null() {
        return String::new();
    }
    let mut len = 0;
    while *p.add(len) != 0 {
        len += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(p, len))
}

pub fn default_printer() -> Option<String> {
    unsafe {
        let mut size: u32 = 0;
        GetDefaultPrinterW(std::ptr::null_mut(), &mut size);
        if size == 0 {
            return None;
        }
        let mut buf = vec![0u16; size as usize];
        if GetDefaultPrinterW(buf.as_mut_ptr(), &mut size) == 0 {
            return None;
        }
        Some(from_wide(buf.as_ptr()))
    }
}

pub fn list_printers() -> Result<Vec<String>, AppError> {
    unsafe {
        let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
        let (mut needed, mut returned) = (0u32, 0u32);
        EnumPrintersW(flags, std::ptr::null(), 4, std::ptr::null_mut(), 0, &mut needed, &mut returned);
        if needed == 0 {
            return Ok(vec![]);
        }
        // مصفوفة u64 لضمان محاذاة المؤشرات داخل البنية.
        let mut buf = vec![0u64; (needed as usize).div_ceil(8)];
        let ok = EnumPrintersW(flags, std::ptr::null(), 4, buf.as_mut_ptr() as *mut u8, needed, &mut needed, &mut returned);
        if ok == 0 {
            return Err(AppError::Internal("تعذر قراءة قائمة الطابعات".into()));
        }
        let infos = std::slice::from_raw_parts(buf.as_ptr() as *const PrinterInfo4W, returned as usize);
        let mut names: Vec<String> = infos.iter().map(|i| from_wide(i.printer_name)).filter(|n| !n.is_empty()).collect();
        names.sort();
        names.dedup();
        Ok(names)
    }
}

pub fn print_raw(printer: &str, bytes: &[u8]) -> Result<(), AppError> {
    let fail = |m: &str| AppError::Internal(format!("تعذرت الطباعة: {m}"));
    unsafe {
        let name = wide(printer);
        let mut handle: *mut c_void = std::ptr::null_mut();
        if OpenPrinterW(name.as_ptr(), &mut handle, std::ptr::null()) == 0 {
            return Err(fail("تعذر فتح الطابعة، تأكد أنها متصلة وشغالة"));
        }
        let mut doc_name = wide("POS");
        let mut datatype = wide("RAW");
        let info = DocInfo1W { doc_name: doc_name.as_mut_ptr(), output_file: std::ptr::null_mut(), datatype: datatype.as_mut_ptr() };
        let result = (|| {
            if StartDocPrinterW(handle, 1, &info) == 0 {
                return Err(fail("رفضت الطابعة بدء المهمة"));
            }
            let mut res = Ok(());
            if StartPagePrinter(handle) == 0 {
                res = Err(fail("رفضت الطابعة بدء الصفحة"));
            } else {
                let mut written: u32 = 0;
                let ok = WritePrinter(handle, bytes.as_ptr() as *const c_void, bytes.len() as u32, &mut written);
                if ok == 0 || written as usize != bytes.len() {
                    res = Err(fail("لم يكتمل إرسال البيانات للطابعة"));
                }
                EndPagePrinter(handle);
            }
            EndDocPrinter(handle);
            res
        })();
        ClosePrinter(handle);
        result
    }
}
