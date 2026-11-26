import { zodResolver } from "@hookform/resolvers/zod";
import { Plus, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { Controller, useFieldArray, useForm } from "react-hook-form";
import Alert from "../../components/Alert";
import { btnGhost, btnPrimary, inputClass } from "../../components/ui";
import { formatMoney, microToInput, milliToInput, minorToInput } from "../../lib/format";
import {
  api,
  type BusinessProfile,
  type LookupItem,
  type PricePreview,
  type ProductInput,
  type Unit,
} from "../../services/api";
import BarcodeInput from "./BarcodeInput";
import ImageField from "./ImageField";
import { emptyProduct, productSchema, type ProductFormValues } from "./schema";

interface Props {
  productId: string | null;
  profile: BusinessProfile | null;
  categories: LookupItem[];
  brands: LookupItem[];
  units: Unit[];
  onSaved: () => void;
  onCancel: () => void;
}

const sectionClass = "space-y-3 rounded-lg border border-slate-200 p-4 dark:border-slate-700";

function Field({ label, error, children }: { label: string; error?: string; children: React.ReactNode }) {
  return (
    <label className="block text-sm">
      {label}
      <div className="mt-1">{children}</div>
      {error && <span role="alert" className="text-xs text-red-600">{error}</span>}
    </label>
  );
}

export default function ProductForm({ productId, profile, categories, brands, units, onSaved, onCancel }: Props) {
  const defaultUnit = units.find((u) => u.code === "piece")?.id ?? units[0]?.id ?? "";
  const {
    register,
    control,
    handleSubmit,
    reset,
    watch,
    setValue,
    formState: { errors, isSubmitting, isDirty },
  } = useForm<ProductFormValues>({ resolver: zodResolver(productSchema), defaultValues: emptyProduct(defaultUnit) });

  const variants = useFieldArray({ control, name: "variants" });
  const extraUnits = useFieldArray({ control, name: "units" });
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(!!productId);
  const [image, setImage] = useState<string | null>(null);
  const [pendingImage, setPendingImage] = useState<string | null>(null);
  const [imageRemoved, setImageRemoved] = useState(false);

  const [editPiece, setEditPiece] = useState(false);
  const [preview, setPreview] = useState<PricePreview | null>(null);
  const baseUnitId = watch("baseUnitId");
  const baseUnit = units.find((u) => u.id === baseUnitId);
  const canWeigh = !!profile?.allowWeightedProducts;
  const canVariants = !!profile?.allowVariants;

  useEffect(() => {
    if (!productId) return;
    let alive = true;
    (async () => {
      try {
        const p = await api.getProduct(productId);
        if (!alive) return;
        reset({
          nameAr: p.nameAr,
          nameEn: p.nameEn,
          sku: p.sku ?? "",
          categoryId: p.categoryId ?? "",
          brandId: p.brandId ?? "",
          baseUnitId: p.baseUnitId,
          purchasePrice: p.units.length > 0 && p.units[0].purchasePriceMinor != null ? "" : minorToInput(p.purchasePrice),
          salePrice: p.priceSource === "derived" ? "" : minorToInput(p.salePrice),
          marginPercent: "",
          buyInBase: p.buyInBase,
          sellInBase: p.sellInBase,
          defaultBuyBig: p.defaultBuyBig,
          defaultSellBig: p.defaultSellBig,
          minStockQty: milliToInput(p.minStockQty),
          isWeighted: p.isWeighted,
          trackExpiry: p.trackExpiry,
          isActive: p.isActive,
          barcodes: p.barcodes,
          variants: p.variants.map((v) => ({
            id: v.id,
            nameAr: v.nameAr,
            nameEn: v.nameEn,
            sku: v.sku ?? "",
            purchasePrice: minorToInput(v.purchasePrice),
            salePrice: minorToInput(v.salePrice),
            isActive: v.isActive,
            barcodes: v.barcodes,
          })),
          units: p.units.map((u) => ({
            unitId: u.unitId,
            factor: microToInput(u.factorMicro),
            isPurchaseUnit: u.isPurchaseUnit,
            isSaleUnit: u.isSaleUnit,
            salePrice: u.salePriceMinor == null ? "" : minorToInput(u.salePriceMinor),
            purchasePrice: u.purchasePriceMinor == null ? "" : minorToInput(u.purchasePriceMinor),
            barcode: u.barcode ?? "",
          })),
        });
        setEditPiece(p.priceSource === "manual");
        if (p.hasImage) setImage(await api.getProductImage(productId));
      } catch (e) {
        setError(String(e));
      } finally {
        if (alive) setLoading(false);
      }
    })();
    return () => {
      alive = false;
    };
  }, [productId, reset]);

  async function onSubmit(v: ProductFormValues) {
    setError(null);
    const input: ProductInput = {
      id: productId ?? undefined,
      sku: v.sku.trim() || null,
      nameAr: v.nameAr,
      nameEn: v.nameEn,
      categoryId: v.categoryId || null,
      brandId: v.brandId || null,
      baseUnitId: v.baseUnitId,
      purchasePrice: v.purchasePrice,
      salePrice: editPiece || v.units.length === 0 ? v.salePrice : "",
      marginPercent: v.marginPercent || null,
      buyInBase: v.buyInBase,
      sellInBase: v.sellInBase,
      defaultBuyBig: v.units.length > 0 && v.defaultBuyBig,
      defaultSellBig: v.units.length > 0 && v.defaultSellBig,
      minStockQty: v.minStockQty || "0",
      isWeighted: v.isWeighted,
      trackExpiry: v.trackExpiry,
      isActive: v.isActive,
      barcodes: v.barcodes,
      variants: v.variants.map((x) => ({ ...x, sku: x.sku.trim() || null })),
      units: v.units,
    };
    try {
      const id = await api.saveProduct(input);
      if (pendingImage) await api.setProductImage(id, pendingImage);
      else if (imageRemoved && productId) await api.removeProductImage(id);
      onSaved();
    } catch (e) {
      setError(String(e));
    }
  }

  const bigUnits = watch("units");
  const hasBig = bigUnits.length > 0;
  const buyInBase = watch("buyInBase");
  const sellInBase = watch("sellInBase");
  const defaultBuyBig = watch("defaultBuyBig");
  const defaultSellBig = watch("defaultSellBig");
  const buyBig = hasBig && bigUnits[0].isPurchaseUnit;
  const sellBig = hasBig && bigUnits[0].isSaleUnit;
  const pieceSale = watch("salePrice");
  const pieceCost = watch("purchasePrice");
  const margin = watch("marginPercent");
  const big0 = bigUnits[0];
  const bigName = units.find((u) => u.id === big0?.unitId)?.nameAr ?? "الوحدة الكبيرة";
  const baseName = baseUnit?.nameAr ?? "قطعة";

  useEffect(() => {
    const t = setTimeout(() => {
      api
        .previewProductPrices({
          factor: hasBig ? big0.factor : undefined,
          pieceSale: editPiece || !hasBig ? pieceSale : "",
          pieceCost,
          bigSale: hasBig ? big0.salePrice : undefined,
          bigPurchase: hasBig ? big0.purchasePrice : undefined,
          marginPercent: margin,
        })
        .then(setPreview)
        .catch(() => setPreview(null));
    }, 200);
    return () => clearTimeout(t);
  }, [hasBig, big0?.factor, big0?.salePrice, big0?.purchasePrice, pieceSale, pieceCost, margin, editPiece]);

  function setBuy(base: boolean, big: boolean, defBig: boolean) {
    if (!base && !big) return;
    setValue("buyInBase", base, { shouldDirty: true });
    setValue("units.0.isPurchaseUnit", big, { shouldDirty: true });
    setValue("defaultBuyBig", !base ? true : !big ? false : defBig, { shouldDirty: true });
  }
  function setSell(base: boolean, big: boolean, defBig: boolean) {
    if (!base && !big) return;
    setValue("sellInBase", base, { shouldDirty: true });
    setValue("units.0.isSaleUnit", big, { shouldDirty: true });
    setValue("defaultSellBig", !base ? true : !big ? false : defBig, { shouldDirty: true });
  }
  function enableBig(on: boolean) {
    if (on) {
      extraUnits.append({ unitId: units.find((u) => u.id !== baseUnitId && u.code === "carton")?.id ?? "", factor: "12", isPurchaseUnit: true, isSaleUnit: true, salePrice: "", purchasePrice: "", barcode: "" });
      setValue("defaultBuyBig", true, { shouldDirty: true });
      setValue("defaultSellBig", false, { shouldDirty: true });
    } else {
      extraUnits.remove(0);
      setValue("buyInBase", true);
      setValue("sellInBase", true);
      setValue("defaultBuyBig", false);
      setValue("defaultSellBig", false);
    }
  }
  const money = (v: number | null | undefined) => (v == null ? "—" : formatMoney(v));

  if (loading) return <p className="py-8 text-center text-sm text-slate-500">جارٍ التحميل…</p>;

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="space-y-4" noValidate>
      {error && <Alert kind="error">{error}</Alert>}

      <section className={sectionClass}>
        <h4 className="font-semibold">البيانات الأساسية</h4>
        <div className="grid gap-3 md:grid-cols-2">
          <Field label="اسم المنتج (عربي) *" error={errors.nameAr?.message}>
            <input autoFocus className={inputClass} {...register("nameAr")} />
          </Field>
          <Field label="اسم المنتج (إنجليزي)">
            <input dir="ltr" className={inputClass} {...register("nameEn")} />
          </Field>
          <Field label="رمز الصنف (SKU)">
            <input dir="ltr" className={inputClass} {...register("sku")} />
          </Field>
          <Field label="الوحدة الأساسية *" error={errors.baseUnitId?.message}>
            <select className={inputClass} {...register("baseUnitId")}>
              {units.map((u) => (
                <option key={u.id} value={u.id}>{u.nameAr}</option>
              ))}
            </select>
          </Field>
          <Field label="التصنيف">
            <select className={inputClass} {...register("categoryId")}>
              <option value="">بدون</option>
              {categories.filter((c) => c.isActive).map((c) => (
                <option key={c.id} value={c.id}>{c.nameAr}</option>
              ))}
            </select>
          </Field>
          <Field label="الماركة">
            <select className={inputClass} {...register("brandId")}>
              <option value="">بدون</option>
              {brands.filter((b) => b.isActive).map((b) => (
                <option key={b.id} value={b.id}>{b.nameAr}</option>
              ))}
            </select>
          </Field>
        </div>
      </section>

      <section className={sectionClass}>
        <h4 className="font-semibold">الشراء والبيع</h4>
        <label className="flex items-center gap-2 text-sm">
          <input type="checkbox" checked={hasBig} onChange={(e) => enableBig(e.target.checked)} /> له وحدة كبيرة (كرتونة، دستة، علبة…)
        </label>
        {hasBig && (
          <div className="grid gap-3 md:grid-cols-[1fr_8rem_1fr]">
            <Field label="الوحدة الكبيرة" error={errors.units?.[0]?.unitId?.message}>
              <select className={inputClass} {...register("units.0.unitId")}>
                <option value="">اختر…</option>
                {units.filter((u) => (u.id !== baseUnitId && ["carton", "dozen", "box", "pack"].includes(u.code)) || u.id === big0.unitId).map((u) => (
                  <option key={u.id} value={u.id}>{u.nameAr}</option>
                ))}
              </select>
            </Field>
            <Field label={`عدد ${baseName} فيها`} error={errors.units?.[0]?.factor?.message}>
              <input inputMode="numeric" className={inputClass} {...register("units.0.factor")} />
            </Field>
            <Field label="باركود الوحدة الكبيرة">
              <input className={inputClass} dir="ltr" {...register("units.0.barcode")} />
            </Field>
          </div>
        )}

        <div className="grid gap-3 md:grid-cols-2">
          <fieldset className="space-y-1 text-sm">
            <legend className="font-medium">بتشتري بـ</legend>
            <label className="flex items-center gap-2">
              <input type="checkbox" checked={buyInBase} onChange={(e) => setBuy(e.target.checked, buyBig, defaultBuyBig)} /> {baseName}
            </label>
            {hasBig && (
              <label className="flex items-center gap-2">
                <input type="checkbox" checked={buyBig} onChange={(e) => setBuy(buyInBase, e.target.checked, defaultBuyBig)} /> {bigName}
              </label>
            )}
            {buyInBase && buyBig && (
              <div className="flex items-center gap-3 pt-1 text-xs text-slate-600 dark:text-slate-300">
                الافتراضي:
                <label className="flex items-center gap-1"><input type="radio" checked={!defaultBuyBig} onChange={() => setBuy(true, true, false)} /> {baseName}</label>
                <label className="flex items-center gap-1"><input type="radio" checked={defaultBuyBig} onChange={() => setBuy(true, true, true)} /> {bigName}</label>
              </div>
            )}
          </fieldset>
          <fieldset className="space-y-1 text-sm">
            <legend className="font-medium">بتبيع بـ</legend>
            <label className="flex items-center gap-2">
              <input type="checkbox" checked={sellInBase} onChange={(e) => setSell(e.target.checked, sellBig, defaultSellBig)} /> {baseName}
            </label>
            {hasBig && (
              <label className="flex items-center gap-2">
                <input type="checkbox" checked={sellBig} onChange={(e) => setSell(sellInBase, e.target.checked, defaultSellBig)} /> {bigName}
              </label>
            )}
            {sellInBase && sellBig && (
              <div className="flex items-center gap-3 pt-1 text-xs text-slate-600 dark:text-slate-300">
                الافتراضي:
                <label className="flex items-center gap-1"><input type="radio" checked={!defaultSellBig} onChange={() => setSell(true, true, false)} /> {baseName}</label>
                <label className="flex items-center gap-1"><input type="radio" checked={defaultSellBig} onChange={() => setSell(true, true, true)} /> {bigName}</label>
              </div>
            )}
          </fieldset>
        </div>

        <div className="grid gap-3 md:grid-cols-4">
          {hasBig && (
            <Field label={`سعر شراء ${bigName} (جملة)`} error={errors.units?.[0]?.purchasePrice?.message}>
              <input inputMode="decimal" className={inputClass} {...register("units.0.purchasePrice")} />
            </Field>
          )}
          {hasBig && (
            <Field label={`سعر بيع ${bigName}`} error={errors.units?.[0]?.salePrice?.message}>
              <input inputMode="decimal" className={inputClass} {...register("units.0.salePrice")} />
            </Field>
          )}
          {(!hasBig || buyInBase) && (
            <Field label={`سعر شراء ${baseName}`} error={errors.purchasePrice?.message}>
              <input inputMode="decimal" className={inputClass} {...register("purchasePrice")} />
            </Field>
          )}
          <Field label="هامش الربح %" error={errors.marginPercent?.message}>
            <input inputMode="decimal" className={inputClass} {...register("marginPercent")} />
          </Field>
        </div>

        <div className="rounded-lg bg-slate-50 p-3 text-sm dark:bg-slate-900/40">
          <div className="flex flex-wrap items-center gap-x-6 gap-y-2">
            <div>
              سعر بيع {baseName}:{" "}
              {editPiece || !hasBig ? (
                <input inputMode="decimal" aria-label="سعر بيع القطعة" className={`${inputClass} !inline-block !w-28`} {...register("salePrice")} />
              ) : (
                <strong>{money(preview?.piecePrice)}</strong>
              )}
              {hasBig && (
                <button type="button" className="ms-2 text-xs text-sky-700 underline dark:text-sky-300" onClick={() => { setEditPiece(!editPiece); if (editPiece) setValue("salePrice", ""); }}>
                  {editPiece ? "حساب تلقائي" : "تعديل"}
                </button>
              )}
            </div>
            {hasBig && <div>تكلفة {baseName}: <strong>{money(preview?.pieceCost)}</strong></div>}
            {hasBig && sellBig && preview?.bigSale != null && <div>سعر بيع {bigName}: <strong>{money(preview.bigSale)}</strong></div>}
          </div>
          {errors.salePrice?.message && <span role="alert" className="text-xs text-red-600">{errors.salePrice.message}</span>}
        </div>

        <div className="grid gap-3 md:grid-cols-3">
          <Field label={`حد إعادة الطلب (${baseName})`} error={errors.minStockQty?.message}>
            <input inputMode="decimal" className={inputClass} {...register("minStockQty")} />
          </Field>
        </div>
        <div className="flex flex-wrap gap-6 text-sm">
          <label className="flex items-center gap-2">
            <input type="checkbox" {...register("isActive")} /> نشط (متاح للبيع)
          </label>
          {canWeigh && (
            <label className="flex items-center gap-2" title={baseUnit?.allowsDecimal ? "" : "يحتاج وحدة تقبل الكسور"}>
              <input type="checkbox" disabled={!baseUnit?.allowsDecimal} {...register("isWeighted")} /> منتج موزون
            </label>
          )}
          {profile?.trackExpiry && (
            <label className="flex items-center gap-2">
              <input type="checkbox" {...register("trackExpiry")} /> تتبع تاريخ الصلاحية
            </label>
          )}
        </div>
      </section>

      <section className={sectionClass}>
        <h4 className="font-semibold">الباركود</h4>
        <Controller control={control} name="barcodes" render={({ field }) => <BarcodeInput value={field.value} onChange={field.onChange} />} />
      </section>

      {(canVariants || variants.fields.length > 0) && (
        <section className={sectionClass}>
          <div className="flex items-center justify-between">
            <h4 className="font-semibold">المتغيرات (أحجام، عبوات…)</h4>
            {canVariants && (
              <button type="button" className={btnGhost} onClick={() => variants.append({ nameAr: "", nameEn: "", sku: "", purchasePrice: "0", salePrice: "0", isActive: true, barcodes: [] })}>
                <Plus size={14} /> إضافة متغير
              </button>
            )}
          </div>
          {variants.fields.map((f, i) => (
            <div key={f.id} className="space-y-3 rounded-lg bg-slate-50 p-3 dark:bg-slate-900/40">
              <div className="grid gap-3 md:grid-cols-4">
                <Field label="الاسم *" error={errors.variants?.[i]?.nameAr?.message}>
                  <input className={inputClass} placeholder="مثال: 100 مل" {...register(`variants.${i}.nameAr`)} />
                </Field>
                <Field label="SKU">
                  <input dir="ltr" className={inputClass} {...register(`variants.${i}.sku`)} />
                </Field>
                <Field label="سعر الشراء" error={errors.variants?.[i]?.purchasePrice?.message}>
                  <input inputMode="decimal" className={inputClass} {...register(`variants.${i}.purchasePrice`)} />
                </Field>
                <Field label="سعر البيع" error={errors.variants?.[i]?.salePrice?.message}>
                  <input inputMode="decimal" className={inputClass} {...register(`variants.${i}.salePrice`)} />
                </Field>
              </div>
              <Controller control={control} name={`variants.${i}.barcodes`} render={({ field }) => <BarcodeInput value={field.value} onChange={field.onChange} />} />
              <div className="flex items-center justify-between text-sm">
                <label className="flex items-center gap-2"><input type="checkbox" {...register(`variants.${i}.isActive`)} /> نشط</label>
                <button type="button" className={`${btnGhost} text-red-600`} onClick={() => variants.remove(i)}>
                  <Trash2 size={14} /> إزالة (يُعطَّل ولا يُحذف)
                </button>
              </div>
            </div>
          ))}
        </section>
      )}

      <section className={sectionClass}>
        <h4 className="font-semibold">الصورة</h4>
        <ImageField
          preview={pendingImage ?? (imageRemoved ? null : image)}
          onPick={(d) => { setPendingImage(d); setImageRemoved(false); }}
          onRemove={() => { setPendingImage(null); setImageRemoved(true); }}
        />
      </section>

      <div className="flex justify-end gap-2">
        <button type="button" className={btnGhost} onClick={() => { if (!isDirty || window.confirm("في تعديلات لم تُحفظ. تخرج بدونها؟")) onCancel(); }}>إلغاء</button>
        <button type="submit" className={btnPrimary} disabled={isSubmitting}>
          {isSubmitting ? "جارٍ الحفظ…" : "حفظ المنتج"}
        </button>
      </div>
    </form>
  );
}
