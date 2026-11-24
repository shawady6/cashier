//! تطبيع النصوص العربية/الإنجليزية للبحث والتحقق من التكرار.

/// يحوّل الأرقام العربية-الهندية إلى أرقام لاتينية.
pub fn ascii_digits(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '٠'..='٩' => char::from(b'0' + (c as u32 - '٠' as u32) as u8),
            '۰'..='۹' => char::from(b'0' + (c as u32 - '۰' as u32) as u8),
            _ => c,
        })
        .collect()
}

/// تطبيع للبحث: إزالة التشكيل والتطويل، توحيد الألف والياء والتاء المربوطة،
/// أرقام لاتينية، أحرف صغيرة، ومسافات مفردة.
pub fn normalize(s: &str) -> String {
    let mapped: String = ascii_digits(s)
        .chars()
        .filter_map(|c| match c {
            '\u{064B}'..='\u{065F}' | '\u{0670}' | '\u{0640}' => None,
            'أ' | 'إ' | 'آ' | 'ٱ' => Some('ا'),
            'ى' => Some('ي'),
            'ة' => Some('ه'),
            _ => Some(c),
        })
        .collect();
    mapped
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unifies_arabic_letter_variants() {
        assert_eq!(normalize("أحمد"), normalize("احمد"));
        assert_eq!(normalize("مَدرَسة"), normalize("مدرسه"));
        assert_eq!(normalize("مستشفى"), normalize("مستشفي"));
    }

    #[test]
    fn converts_digits_and_case() {
        assert_eq!(normalize("  Persil  ٣ كيلو "), "persil 3 كيلو");
    }
}
