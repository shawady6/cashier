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
