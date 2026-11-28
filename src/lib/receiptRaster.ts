import { formatMoney, formatQty } from "./format";
import type { SaleDetail } from "../services/api";
import type { ReceiptSettings } from "../components/Receipt";

export interface RasterPayload {
  width: number;
  height: number;
  dataBase64: string;
}

type Align = "right" | "left" | "center";
interface Line {
  start?: string;
  end?: string;
  center?: string;
  bold?: boolean;
  big?: boolean;
  rule?: boolean;
  gap?: number;
}

export function dotsFor(paper: "58" | "80"): number {
  return paper === "58" ? 384 : 576;
}

const FONT = '"Segoe UI", Tahoma, Arial, sans-serif';

/** يرسم الأسطر على canvas بعرض الطابعة ثم يحوّلها إلى بتات أحادية (1 = أسود). الأرقام جاهزة من Rust. */
export function renderLines(lines: Line[], width: number): RasterPayload {
  const pad = 8;
  const probe = document.createElement("canvas").getContext("2d");
  if (!probe) throw new Error("تعذر تجهيز الرسم");
  const size = (l: Line) => (l.big ? 30 : 24);
  const lh = (l: Line) => (l.rule ? 12 : l.gap ? l.gap : Math.round(size(l) * 1.45));
  const height = Math.max(8, lines.reduce((a, l) => a + lh(l), 0) + 16);

  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) throw new Error("تعذر تجهيز الرسم");
  ctx.fillStyle = "#fff";
  ctx.fillRect(0, 0, width, height);
  ctx.fillStyle = "#000";
  ctx.textBaseline = "middle";

  let y = 8;
  const put = (text: string, align: Align, l: Line, cy: number) => {
    ctx.font = `${l.bold || l.big ? "bold " : ""}${size(l)}px ${FONT}`;
    ctx.direction = "rtl";
    ctx.textAlign = align;
    const x = align === "right" ? width - pad : align === "left" ? pad : width / 2;
    ctx.fillText(text, x, cy);
  };
  for (const l of lines) {
    const h = lh(l);
    const cy = y + h / 2;
    if (l.rule) {
      ctx.setLineDash([6, 4]);
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(pad, cy);
      ctx.lineTo(width - pad, cy);
      ctx.stroke();
    } else {
      if (l.center) put(l.center, "center", l, cy);
      if (l.start) put(l.start, "right", l, cy);
      if (l.end) put(l.end, "left", l, cy);
    }
    y += h;
  }

  const rgba = ctx.getImageData(0, 0, width, height).data;
  const rowBytes = width / 8;
  const bits = new Uint8Array(rowBytes * height);
  for (let row = 0; row < height; row++) {
    for (let x = 0; x < width; x++) {
      const i = (row * width + x) * 4;
      const lum = rgba[i] * 0.299 + rgba[i + 1] * 0.587 + rgba[i + 2] * 0.114;
      if (lum < 140) bits[row * rowBytes + (x >> 3)] |= 0x80 >> (x & 7);
    }
  }
  let bin = "";
  for (let i = 0; i < bits.length; i += 0x8000) bin += String.fromCharCode(...bits.subarray(i, i + 0x8000));
  return { width, height, dataBase64: btoa(bin) };
}

const methodLabel: Record<string, string> = { cash: "نقدًا", card: "كارت" };

export function receiptLines(sale: SaleDetail, s: ReceiptSettings): Line[] {
  const L: Line[] = [{ center: s.storeName || "المتجر", big: true }];
  if (s.address) L.push({ center: s.address });
  if (s.phone) L.push({ center: s.phone });
  L.push({ rule: true });
  L.push({ start: "فاتورة", end: sale.invoiceNo });
  L.push({ start: "التاريخ", end: new Date(sale.createdAt).toLocaleString("ar-EG") });
  L.push({ rule: true });
  for (const i of sale.items) {
    L.push({ start: `${i.nameAr}${i.variantName ? ` - ${i.variantName}` : ""}`, bold: true });
    L.push({ start: `${formatQty(i.qtyMilli)} ${i.unitName} × ${formatMoney(i.unitPriceMinor)}`, end: formatMoney(i.grossMinor) });
    if (i.discountMinor > 0) L.push({ start: "خصم", end: `-${formatMoney(i.discountMinor)}` });
  }
  L.push({ rule: true });
  L.push({ start: "الإجمالي قبل الخصم", end: formatMoney(sale.subtotalMinor) });
  const disc = sale.itemDiscountMinor + sale.invoiceDiscountMinor;
  if (disc > 0) L.push({ start: "إجمالي الخصم", end: `-${formatMoney(disc)}` });
  if (sale.taxMinor > 0) L.push({ start: "شامل الضريبة", end: formatMoney(sale.taxMinor) });
  L.push({ start: "الإجمالي", end: formatMoney(sale.totalMinor), big: true });
  L.push({ rule: true });
  for (const p of sale.payments) L.push({ start: methodLabel[p.method] ?? p.method, end: formatMoney(p.amountMinor) });
  if (sale.dueMinor > 0) L.push({ start: "آجل على العميل", end: formatMoney(sale.dueMinor), bold: true });
  if (sale.customerName) L.push({ start: `العميل: ${sale.customerName}` });
  if (sale.changeMinor > 0) L.push({ start: "الباقي", end: formatMoney(sale.changeMinor), bold: true });
  if (s.footer) {
    L.push({ rule: true });
    L.push({ center: s.footer });
  }
  return L;
}

export function renderReceipt(sale: SaleDetail, s: ReceiptSettings): RasterPayload {
  return renderLines(receiptLines(sale, s), dotsFor(s.paperWidth));
}

export function renderTestPage(s: ReceiptSettings): RasterPayload {
  const L: Line[] = [
    { center: s.storeName || "المتجر", big: true },
    { center: "طباعة تجريبية" },
    { rule: true },
    { start: "الصنف", end: "السعر" },
    { start: "سكر ١ كجم", end: formatMoney(4500) },
    { start: "شاي ٢٥ فتلة", end: formatMoney(6250) },
    { rule: true },
    { start: "الإجمالي", end: formatMoney(10750), big: true },
    { rule: true },
    { center: new Date().toLocaleString("ar-EG") },
    { center: "0123456789 ABC abc" },
  ];
  return renderLines(L, dotsFor(s.paperWidth));
}
