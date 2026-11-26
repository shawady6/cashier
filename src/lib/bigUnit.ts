// مساعدات عرض فقط للوحدة الكبيرة. المخزون والتكلفة وتحويل الوحدات كلها في Rust؛
// الأرقام هنا للمعاينة قبل الحفظ وتعبئة القيم الافتراضية القابلة للتعديل.
import { minorToInput } from "./format";
import type { ProductDetail, Unit } from "../services/api";

export interface UnitChoice {
  unitId: string;
  nameAr: string;
  /** عدد القطع الافتراضي في الوحدة (1 للقطعة) */
  factor: number;
  isBase: boolean;
  /** اسم الوحدة الأساسية (القطعة) */
  baseName: string;
  /** تكلفة الوحدة المقترحة بالقروش */
  defaultCostMinor: number;
  /** سعر بيع الوحدة الحالي/المقترح بالقروش */
  defaultSaleMinor: number;
}

export interface UnitLineState {
  unitId: string;
  qty: string;
  /** عدد القطع في الوحدة لهذا البند فقط */
  factor: string;
  cost: string;
  salePrice: string;
  /** السعر المقترح وقت اختيار الوحدة، لمعرفة هل غيّره المستخدم */
  origSalePrice: string;
}

/** الوحدات المتاحة للسطر: الافتراضية أولًا. في الشراء تُحترم خيارات "بتشتري بـ". */
export function buildChoices(detail: ProductDetail, units: Unit[], variantId: string | null, purchaseOnly: boolean): UnitChoice[] {
  const v = variantId ? detail.variants.find((x) => x.id === variantId) : undefined;
  const baseCost = v ? v.purchasePrice : detail.purchasePrice;
  const baseSale = v ? v.salePrice : detail.salePrice;
  const baseName = units.find((u) => u.id === detail.baseUnitId)?.nameAr ?? "قطعة";
  const out: UnitChoice[] = [];
  if (!purchaseOnly || detail.buyInBase) {
    out.push({ unitId: detail.baseUnitId, nameAr: baseName, factor: 1, isBase: true, baseName, defaultCostMinor: baseCost, defaultSaleMinor: baseSale });
  }
  for (const u of detail.units) {
    if (purchaseOnly && !u.isPurchaseUnit) continue;
    const factor = u.factorMicro / 1_000_000;
    out.push({
      unitId: u.unitId,
      nameAr: units.find((x) => x.id === u.unitId)?.nameAr ?? "وحدة",
      factor,
      isBase: false,
      baseName,
      defaultCostMinor: u.purchasePriceMinor ?? Math.round(baseCost * factor),
      defaultSaleMinor: !v && u.salePriceMinor != null ? u.salePriceMinor : Math.round(baseSale * factor),
    });
  }
  if (detail.defaultBuyBig) out.sort((a, b) => Number(a.isBase) - Number(b.isBase));
  return out;
}

export function lineForChoice(c: UnitChoice, qty = "1"): UnitLineState {
  const sale = minorToInput(c.defaultSaleMinor);
  return { unitId: c.unitId, qty, factor: String(c.factor), cost: minorToInput(c.defaultCostMinor), salePrice: sale, origSalePrice: sale };
}

const toNum = (s: string): number => {
  const n = Number(s.trim().replace(/[٠-٩]/g, (d) => String("٠١٢٣٤٥٦٧٨٩".indexOf(d))).replace(/[,٫]/g, "."));
  return Number.isFinite(n) ? n : NaN;
};

export interface LinePreview {
  basePieces: number;
  costPerPiece: number;
  below: boolean;
  priceChanges: boolean;
}

export function previewLine(c: UnitChoice | undefined, s: UnitLineState, isVariant: boolean): LinePreview | null {
  if (!c) return null;
  const qty = toNum(s.qty);
  const cost = toNum(s.cost);
  const factor = c.isBase ? 1 : toNum(s.factor);
  if (!(qty > 0) || !(factor >= 1)) return null;
  const basePieces = qty * factor;
  const costPerPiece = Number.isFinite(cost) ? (cost * qty) / basePieces : NaN;
  const sale = toNum(s.salePrice);
  return {
    basePieces,
    costPerPiece,
    below: Number.isFinite(sale) && Number.isFinite(cost) && s.salePrice.trim() !== "" && sale < cost,
    priceChanges: s.salePrice.trim() !== "" && s.salePrice.trim() !== s.origSalePrice && !(isVariant && !c.isBase),
  };
}

export const fmt2 = (n: number) => (Number.isFinite(n) ? n.toFixed(2) : "—");
