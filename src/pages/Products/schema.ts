import { z } from "zod";

// تحقق شكلي سريع للمستخدم فقط. التحقق الحاكم والتحليل الفعلي للأرقام في Rust.
const num = (label: string) =>
  z.string().trim().regex(/^[0-9٠-٩]+([.,٫][0-9٠-٩]+)?$/, `${label} غير صالح`);

const optionalNum = (label: string) =>
  z.string().trim().refine((v) => v === "" || /^[0-9٠-٩]+([.,٫][0-9٠-٩]+)?$/.test(v), `${label} غير صالح`);

export const variantSchema = z.object({
  id: z.string().optional(),
  nameAr: z.string().trim().min(1, "اسم المتغير مطلوب"),
  nameEn: z.string(),
  sku: z.string(),
  purchasePrice: num("سعر الشراء"),
  salePrice: num("سعر البيع"),
  isActive: z.boolean(),
  barcodes: z.array(z.string()),
});

export const productSchema = z.object({
  nameAr: z.string().trim().min(1, "اسم المنتج بالعربية مطلوب"),
  nameEn: z.string(),
  sku: z.string(),
  categoryId: z.string(),
  brandId: z.string(),
  baseUnitId: z.string().min(1, "اختر الوحدة الأساسية"),
  purchasePrice: optionalNum("سعر الشراء"),
  salePrice: optionalNum("سعر بيع القطعة"),
  marginPercent: optionalNum("هامش الربح"),
  buyInBase: z.boolean(),
  sellInBase: z.boolean(),
  defaultBuyBig: z.boolean(),
  defaultSellBig: z.boolean(),
  minStockQty: optionalNum("الحد الأدنى"),
  isWeighted: z.boolean(),
  trackExpiry: z.boolean(),
  isActive: z.boolean(),
  barcodes: z.array(z.string()),
  variants: z.array(variantSchema),
  units: z.array(
    z.object({
      unitId: z.string().min(1, "اختر الوحدة"),
      factor: z.string().trim().regex(/^[0-9٠-٩]+$/, "عدد القطع رقم صحيح").refine((v) => Number(v.replace(/[٠-٩]/g, (d) => String("٠١٢٣٤٥٦٧٨٩".indexOf(d)))) >= 2, "عدد القطع 2 أو أكثر"),
      isPurchaseUnit: z.boolean(),
      isSaleUnit: z.boolean(),
      salePrice: optionalNum("سعر بيع الوحدة"),
      purchasePrice: optionalNum("سعر شراء الوحدة"),
      barcode: z.string(),
    }),
  ),
});

export type ProductFormValues = z.infer<typeof productSchema>;

export const emptyProduct = (baseUnitId: string): ProductFormValues => ({
  nameAr: "",
  nameEn: "",
  sku: "",
  categoryId: "",
  brandId: "",
  baseUnitId,
  purchasePrice: "",
  salePrice: "",
  marginPercent: "",
  buyInBase: true,
  sellInBase: true,
  defaultBuyBig: false,
  defaultSellBig: false,
  minStockQty: "0",
  isWeighted: false,
  trackExpiry: false,
  isActive: true,
  barcodes: [],
  variants: [],
  units: [],
});
