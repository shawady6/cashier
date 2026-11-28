use super::*;

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[test]
fn drawer_pulse_bytes() {
    assert_eq!(escpos::drawer_pulse(0), vec![0x1B, 0x70, 0, 25, 250]);
    assert_eq!(escpos::drawer_pulse(1), vec![0x1B, 0x70, 1, 25, 250]);
    assert_eq!(escpos::drawer_pulse(7)[2], 1, "أي قيمة أخرى تُختزل إلى 0 أو 1");
}

#[test]
fn raster_header_and_chunking() {
    // 384 نقطة = 48 بايت في الصف، 300 صف: شرائح 128 + 128 + 44
    let bits = vec![0xAAu8; 48 * 300];
    let out = escpos::raster(384, 300, &bits);
    assert_eq!(&out[..8], &[0x1D, 0x76, 0x30, 0, 48, 0, 128, 0]);
    let second = 8 + 48 * 128;
    assert_eq!(&out[second..second + 8], &[0x1D, 0x76, 0x30, 0, 48, 0, 128, 0]);
    let third = second + 8 + 48 * 128;
    assert_eq!(&out[third..third + 8], &[0x1D, 0x76, 0x30, 0, 48, 0, 44, 0]);
    assert_eq!(out.len(), third + 8 + 48 * 44);
}

#[test]
fn raster_80mm_row_width_uses_two_bytes_when_needed() {
    let out = escpos::raster(576, 1, &vec![0u8; 72]);
    assert_eq!(&out[4..8], &[72, 0, 1, 0]);
}

#[test]
fn job_order_drawer_then_image_then_cut() {
    let bits = vec![0xFFu8; 48 * 2];
    let job = escpos::Job { image: Some((384, 2, &bits)), cut: true, drawer_pin: Some(0) };
    let out = escpos::build_job(&job);
    assert_eq!(&out[..2], &[0x1B, 0x40]);
    assert_eq!(&out[2..7], &[0x1B, 0x70, 0, 25, 250]);
    assert_eq!(&out[7..10], &[0x1D, 0x76, 0x30]);
    assert_eq!(&out[out.len() - 4..], &[0x1D, 0x56, 0x42, 0x00]);

    let no_cut = escpos::build_job(&escpos::Job { image: Some((384, 2, &bits)), cut: false, drawer_pin: None });
    assert!(!no_cut.windows(3).any(|w| w == [0x1D, 0x56, 0x42]));
    assert!(!no_cut.windows(2).any(|w| w == [0x1B, 0x70]));
}

#[test]
fn drawer_only_job_has_no_image() {
    let out = escpos::build_job(&escpos::Job { image: None, cut: true, drawer_pin: Some(1) });
    assert_eq!(out, vec![0x1B, 0x40, 0x1B, 0x70, 1, 25, 250]);
}

#[test]
fn raster_input_validation() {
    let ok = RasterInput { width: 576, height: 2, data_base64: b64(&vec![0u8; 72 * 2]) };
    assert_eq!(decode_raster(&ok, "80").unwrap().len(), 144);

    let wrong_width = RasterInput { width: 384, height: 2, data_base64: b64(&vec![0u8; 48 * 2]) };
    assert!(decode_raster(&wrong_width, "80").is_err());
    assert!(decode_raster(&wrong_width, "58").is_ok());

    let wrong_len = RasterInput { width: 576, height: 2, data_base64: b64(&[0u8; 10]) };
    assert!(decode_raster(&wrong_len, "80").is_err());

    let garbage = RasterInput { width: 576, height: 2, data_base64: "***".into() };
    assert!(decode_raster(&garbage, "80").is_err());

    let zero = RasterInput { width: 576, height: 0, data_base64: String::new() };
    assert!(decode_raster(&zero, "80").is_err());
    let huge = RasterInput { width: 576, height: MAX_ROWS + 1, data_base64: b64(&vec![0u8; 72 * (MAX_ROWS + 1)]) };
    assert!(decode_raster(&huge, "80").is_err());
}

#[test]
fn printing_requires_an_installed_selected_printer() {
    let mut s: HashMap<String, String> = HashMap::new();
    s.insert("receipt_paper_width".into(), "80".into());
    let img = RasterInput { width: 576, height: 1, data_base64: b64(&[0u8; 72]) };
    assert!(print_image(&s, &img, false).is_err(), "بدون طابعة مختارة");
    s.insert("printer_name".into(), "طابعة غير موجودة".into());
    assert!(print_image(&s, &img, false).is_err());
    assert!(open_drawer(&s).is_err());
}

#[test]
fn drawer_pin_setting() {
    let mut s: HashMap<String, String> = HashMap::new();
    assert_eq!(drawer_pin(&s), 0);
    s.insert("drawer_pin".into(), "1".into());
    assert_eq!(drawer_pin(&s), 1);
}
