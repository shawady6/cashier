// تحرير نص الكمية في السلة فقط (+1 / -1). التحقق والحساب الفعلي في Rust.
// نستخدم أعدادًا صحيحة بالألف لتفادي أخطاء الفاصلة العائمة.

export function qtyToMilli(s: string): number | null {
  const m = /^(\d+)(?:\.(\d{1,3}))?$/.exec(s.trim());
  if (!m) return null;
  return Number(m[1]) * 1000 + Number((m[2] ?? "").padEnd(3, "0") || 0);
}

export function milliToQty(m: number): string {
  const whole = Math.trunc(m / 1000);
  const frac = String(m % 1000).padStart(3, "0").replace(/0+$/, "");
  return frac ? `${whole}.${frac}` : String(whole);
}

export function stepQty(s: string, delta: number): string {
  const m = qtyToMilli(s) ?? 0;
  return milliToQty(Math.max(0, m + delta * 1000));
}
