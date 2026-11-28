//! بناء أوامر ESC/POS للطابعات الحرارية. دوال نقية قابلة للاختبار بدون جهاز.

const ESC: u8 = 0x1B;
const GS: u8 = 0x1D;
/// الصور تُقسَّم لشرائح حتى لا تفشل طابعات تقبل ارتفاعًا محدودًا لكل أمر.
const CHUNK_ROWS: usize = 128;

pub fn init() -> Vec<u8> {
    vec![ESC, 0x40]
}

pub fn feed(lines: u8) -> Vec<u8> {
    vec![ESC, 0x64, lines]
}

/// قص جزئي بعد تغذية.
pub fn cut() -> Vec<u8> {
    vec![GS, 0x56, 0x42, 0x00]
}

/// نبضة فتح الدُرج: ESC p pin t1 t2 (الوحدة 2 مللي ثانية). pin 0 أو 1.
pub fn drawer_pulse(pin: u8) -> Vec<u8> {
    vec![ESC, 0x70, pin & 1, 25, 250]
}

/// صورة أحادية اللون (1 = أسود) بعرض مضاعفات 8 نقاط، مقسّمة لأوامر GS v 0.
pub fn raster(width_dots: usize, height: usize, bits: &[u8]) -> Vec<u8> {
    let bytes_per_row = width_dots / 8;
    let mut out = Vec::with_capacity(bits.len() + 16 * (height / CHUNK_ROWS + 1));
    let mut row = 0;
    while row < height {
        let rows = CHUNK_ROWS.min(height - row);
        out.extend_from_slice(&[
            GS,
            0x76,
            0x30,
            0x00,
            (bytes_per_row & 0xFF) as u8,
            (bytes_per_row >> 8) as u8,
            (rows & 0xFF) as u8,
            (rows >> 8) as u8,
        ]);
        out.extend_from_slice(&bits[row * bytes_per_row..(row + rows) * bytes_per_row]);
        row += rows;
    }
    out
}

pub struct Job<'a> {
    pub image: Option<(usize, usize, &'a [u8])>,
    pub cut: bool,
    pub drawer_pin: Option<u8>,
}

/// فتح الدُرج أولًا (يفتح فورًا)، ثم الصورة، ثم التغذية والقص.
pub fn build_job(job: &Job) -> Vec<u8> {
    let mut out = init();
    if let Some(pin) = job.drawer_pin {
        out.extend(drawer_pulse(pin));
    }
    if let Some((w, h, bits)) = job.image {
        out.extend(raster(w, h, bits));
        out.extend(feed(if job.cut { 4 } else { 6 }));
        if job.cut {
            out.extend(cut());
        }
    }
    out
}
