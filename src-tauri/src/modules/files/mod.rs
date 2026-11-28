//! مجلد البرنامج الثابت في «المستندات»: كل ما يخرج من البرنامج (نسخ احتياطية، تقارير) يوضع فيه
//! ليكون معروفًا للمستخدم. المسارات هنا تُحسب في الباك إند فقط ولا تُستقبل من الواجهة.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const ROOT_NAME: &str = "POS System";
pub const BACKUPS_NAME: &str = "Backups";
pub const REPORTS_NAME: &str = "Reports";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppFolder {
    Root,
    Backups,
    Reports,
}

impl AppFolder {
    pub fn parse(s: &str) -> Option<AppFolder> {
        match s {
            "root" => Some(AppFolder::Root),
            "backups" => Some(AppFolder::Backups),
            "reports" => Some(AppFolder::Reports),
            _ => None,
        }
    }
}

/// `documents` هو مجلد المستندات في ويندوز. إن لم يتوفر نستخدم مجلد بيانات البرنامج.
pub fn root(documents: Option<&Path>, data_dir: &Path) -> PathBuf {
    match documents {
        Some(d) => d.join(ROOT_NAME),
        None => data_dir.join("files"),
    }
}

pub fn path_of(documents: Option<&Path>, data_dir: &Path, folder: AppFolder) -> PathBuf {
    let r = root(documents, data_dir);
    match folder {
        AppFolder::Root => r,
        AppFolder::Backups => r.join(BACKUPS_NAME),
        AppFolder::Reports => r.join(REPORTS_NAME),
    }
}

pub fn ensure(documents: Option<&Path>, data_dir: &Path, folder: AppFolder) -> std::io::Result<PathBuf> {
    let p = path_of(documents, data_dir, folder);
    std::fs::create_dir_all(&p)?;
    Ok(p)
}

/// مجلد النسخ الإضافي الفعلي: ما اختاره المستخدم، وإلا مجلد البرنامج في المستندات. فارغ = لا نسخ إضافي.
pub fn effective_backup_dir(settings: &HashMap<String, String>, documents: Option<&Path>, data_dir: &Path) -> String {
    let chosen = settings.get("backup_external_dir").map(|s| s.trim()).unwrap_or("");
    if !chosen.is_empty() {
        return chosen.to_string();
    }
    match (documents, ensure(documents, data_dir, AppFolder::Backups)) {
        (Some(_), Ok(p)) => p.to_string_lossy().to_string(),
        _ => String::new(),
    }
}

/// يبقي الحروف والأرقام و - _ . فقط، حتى لا يخرج اسم ملف خطير.
pub fn sanitize_name(raw: &str) -> String {
    let s: String = raw.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' }).collect();
    let s = s.trim_matches('.').to_string();
    if s.is_empty() {
        "file".to_string()
    } else {
        s.chars().take(80).collect()
    }
}

/// اسم ملف غير مستخدم داخل المجلد: base.ext ثم base-2.ext ...
pub fn unique_path(dir: &Path, base: &str, ext: &str) -> PathBuf {
    let base = sanitize_name(base);
    let mut p = dir.join(format!("{base}.{ext}"));
    let mut n = 1;
    while p.exists() {
        n += 1;
        p = dir.join(format!("{base}-{n}.{ext}"));
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let p = std::env::temp_dir().join(format!("pos-files-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn folders_live_under_documents_or_fall_back_to_data_dir() {
        let docs = PathBuf::from("/docs");
        let data = PathBuf::from("/data");
        assert_eq!(path_of(Some(&docs), &data, AppFolder::Root), PathBuf::from("/docs/POS System"));
        assert_eq!(path_of(Some(&docs), &data, AppFolder::Backups), PathBuf::from("/docs/POS System/Backups"));
        assert_eq!(path_of(Some(&docs), &data, AppFolder::Reports), PathBuf::from("/docs/POS System/Reports"));
        assert_eq!(path_of(None, &data, AppFolder::Reports), PathBuf::from("/data/files/Reports"));
        assert_eq!(AppFolder::parse("reports"), Some(AppFolder::Reports));
        assert_eq!(AppFolder::parse("../etc"), None);
    }

    #[test]
    fn ensure_creates_the_folder() {
        let d = tmp();
        let p = ensure(Some(&d), &d, AppFolder::Reports).unwrap();
        assert!(p.is_dir() && p.ends_with("POS System/Reports"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn backup_dir_prefers_user_choice_then_default() {
        let d = tmp();
        let mut s = HashMap::new();
        let def = effective_backup_dir(&s, Some(&d), &d);
        assert!(def.ends_with("POS System/Backups") && Path::new(&def).is_dir());
        s.insert("backup_external_dir".into(), "  E:\\Mine ".into());
        assert_eq!(effective_backup_dir(&s, Some(&d), &d), "E:\\Mine");
        assert_eq!(effective_backup_dir(&HashMap::new(), None, &d), "", "بدون مستندات لا نسخ إضافي افتراضي");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn names_are_sanitized_and_unique() {
        assert_eq!(sanitize_name("../../x y.csv"), "_.._x_y.csv");
        assert!(!sanitize_name("a/b\\c:d").contains(['/', '\\', ':']));
        assert_eq!(sanitize_name("..."), "file");
        let d = tmp();
        let a = unique_path(&d, "sales_2026", "csv");
        std::fs::write(&a, "x").unwrap();
        let b = unique_path(&d, "sales_2026", "csv");
        assert_ne!(a, b);
        assert!(b.to_string_lossy().ends_with("sales_2026-2.csv"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
