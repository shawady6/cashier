import { zodResolver } from "@hookform/resolvers/zod";
import { Plus, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { Controller, useFieldArray, useForm } from "react-hook-form";
import Alert from "../../components/Alert";
import { btnGhost, btnPrimary, inputClass } from "../../components/ui";
import { microToInput, milliToInput, minorToInput } from "../../lib/format";
import {
  api,
  type BusinessProfile,
  type LookupItem,
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
    formState: { errors, isSubmitting },
  } = useForm<ProductFormValues>({ resolver: zodResolver(productSchema), defaultValues: emptyProduct(defaultUnit) });

  const variants = useFieldArray({ control, name: "variants" });
  const extraUnits = useFieldArray({ control, name: "units" });
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(!!productId);
  const [image, setImage] = useState<string | null>(null);
  const [pendingImage, setPendingImage] = useState<string | null>(null);
  const [imageRemoved, setImageRemoved] = useState(false);

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
          purchasePrice: minorToInput(p.purchasePrice),
          salePrice: minorToInput(p.salePrice),
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
            barcode: u.barcode ?? "",
          })),
        });
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
      salePrice: v.salePrice,
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
        <h4 className="font-semibold">الأسعار والمخزون</h4>
        <div className="grid gap-3 md:grid-cols-3">
          <Field label="سعر الشراء (جنيه) *" error={errors.purchasePrice?.message}>
            <input inputMode="decimal" className={inputClass} {...register("purchasePrice")} />
          </Field>
          <Field label="سعر البيع (جنيه) *" error={errors.salePrice?.message}>
            <input inputMode="decimal" className={inputClass} {...register("salePrice")} />
          </Field>
          <Field
            label={`حد إعادة الطلب (${baseUnit?.nameAr ?? ""})`}
            error={errors.minStockQty?.message}
          >
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

      <section className={sectionClass}>
        <div className="flex items-center justify-between">
          <h4 className="font-semibold">الوحدة الكبيرة (كرتونة، دستة…)</h4>
          {extraUnits.fields.length === 0 && (
            <button type="button" className={btnGhost} onClick={() => extraUnits.append({ unitId: "", factor: "12", isPurchaseUnit: true, isSaleUnit: true, salePrice: "", barcode: "" })}>
              <Plus size={14} /> إضافة وحدة كبيرة
            </button>
          )}
        </div>
        {extraUnits.fields.length === 0 && <p className="text-xs text-slate-500">اختيارية. المخزون يُحفظ دائمًا بالقطعة، والوحدة الكبيرة للشراء والبيع فقط.</p>}
        {extraUnits.fields.map((f, i) => {
          const pieceSale = Number((watch("salePrice") || "0").replace(/[٠-٩]/g, (d) => String("٠١٢٣٤٥٦٧٨٩".indexOf(d))).replace(/[,٫]/g, "."));
          const count = Number((watch(`units.${i}.factor`) || "0").replace(/[٠-٩]/g, (d) => String("٠١٢٣٤٥٦٧٨٩".indexOf(d))));
          const hint = Number.isFinite(pieceSale) && count >= 2 ? (Math.round(pieceSale * 100 * count) / 100).toFixed(2) : "";
          return (
            <div key={f.id} className="grid items-end gap-2 md:grid-cols-[1fr_8rem_1fr_1fr_auto]">
              <Field label="الوحدة الكبيرة" error={errors.units?.[i]?.unitId?.message}>
                <select className={inputClass} {...register(`units.${i}.unitId`)}>
                  <option value="">اختر…</option>
                  {units.filter((u) => u.id !== baseUnitId && ["carton", "dozen", "box", "pack"].includes(u.code) || u.id === f.unitId).map((u) => (
                    <option key={u.id} value={u.id}>{u.nameAr}</option>
                  ))}
                </select>
              </Field>
              <Field label={`عدد ${baseUnit?.nameAr ?? "القطع"} فيها`} error={errors.units?.[i]?.factor?.message}>
                <input inputMode="numeric" className={inputClass} {...register(`units.${i}.factor`)} />
              </Field>
              <Field label="سعر بيع الوحدة الكبيرة" error={errors.units?.[i]?.salePrice?.message}>
                <input inputMode="decimal" className={inputClass} placeholder={hint ? `اقتراح: ${hint}` : "اختياري"} {...register(`units.${i}.salePrice`)} />
              </Field>
              <Field label="باركود الوحدة الكبيرة (اختياري)">
                <input className={inputClass} dir="ltr" {...register(`units.${i}.barcode`)} />
              </Field>
              <button type="button" aria-label="حذف الوحدة الكبيرة" className={`${btnGhost} mb-0.5 text-red-600`} onClick={() => extraUnits.remove(i)}>
                <Trash2 size={14} />
              </button>
            </div>
          );
        })}
        {extraUnits.fields.length > 0 && <p className="text-xs text-slate-500">لو تركت السعر فارغًا يُحسب سعر القطعة × العدد.</p>}
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
        <button type="button" className={btnGhost} onClick={onCancel}>إلغاء</button>
        <button type="submit" className={btnPrimary} disabled={isSubmitting}>
          {isSubmitting ? "جارٍ الحفظ…" : "حفظ المنتج"}
        </button>
      </div>
    </form>
  );
}
