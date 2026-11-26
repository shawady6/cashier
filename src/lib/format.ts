// عرض فقط. لا حسابات مالية هنا: القيم تصل من Rust بالقروش/الألف جاهزة.

const money = new Intl.NumberFormat("ar-EG", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
const qty = new Intl.NumberFormat("ar-EG", { minimumFractionDigits: 0, maximumFractionDigits: 3 });

export function formatMoney(minor: number): string {
  return `${money.format(minor / 100)} ج.م`;
}

export function formatQty(milli: number): string {
  return qty.format(milli / 1000);
}

/** نص صالح للإدخال (لاتيني) من قروش، لتعبئة حقول التعديل فقط. */
export function minorToInput(minor: number): string {
  const whole = Math.trunc(minor / 100);
  const frac = String(minor % 100).padStart(2, "0");
  return frac === "00" ? String(whole) : `${whole}.${frac}`;
}

export function milliToInput(milli: number): string {
  const whole = Math.trunc(milli / 1000);
  const frac = String(milli % 1000).padStart(3, "0").replace(/0+$/, "");
  return frac ? `${whole}.${frac}` : String(whole);
}

export function microToInput(micro: number): string {
  const whole = Math.trunc(micro / 1_000_000);
  const frac = String(micro % 1_000_000).padStart(6, "0").replace(/0+$/, "");
  return frac ? `${whole}.${frac}` : String(whole);
}

/**
 * عرض الرصيد بالوحدة الكبيرة: «5 كرتونة و 7 قطعة» (عرض فقط بالمعامل الافتراضي للمنتج).
 * يرجع نصًا فارغًا لو لا توجد وحدة كبيرة أو الكمية كسرية أو أقل من وحدة كاملة.
 */
export function formatBigUnitStock(
  qtyMilli: number,
  big: { nameAr: string; factorMicro: number } | undefined,
  baseName: string,
): string {
  if (!big || big.factorMicro < 2_000_000 || big.factorMicro % 1_000_000 !== 0) return "";
  if (qtyMilli % 1000 !== 0 || qtyMilli <= 0) return "";
  const pieces = qtyMilli / 1000;
  const factor = big.factorMicro / 1_000_000;
  const bigCount = Math.floor(pieces / factor);
  if (bigCount === 0) return "";
  const rest = pieces % factor;
  return rest === 0 ? `${bigCount} ${big.nameAr}` : `${bigCount} ${big.nameAr} و ${rest} ${baseName}`;
}
