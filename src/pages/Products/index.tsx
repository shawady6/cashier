import { Archive, ArchiveRestore, Boxes, Pencil, Plus, Search } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import Alert from "../../components/Alert";
import Modal from "../../components/Modal";
import PageHeader from "../../components/PageHeader";
import { btnGhost, btnPrimary, cardClass, inputClass } from "../../components/ui";
import { formatMoney, formatQty } from "../../lib/format";
import {
  api,
  type BusinessProfile,
  type LookupItem,
  type ProductListItem,
  type Unit,
} from "../../services/api";
import { useSettingsStore } from "../../store/settingsStore";
import ProductForm from "./ProductForm";
import StockDialog from "./StockDialog";

export default function ProductsPage() {
  const profileCode = useSettingsStore((s) => s.settings.business_profile_code);
  const [products, setProducts] = useState<ProductListItem[]>([]);
  const [categories, setCategories] = useState<LookupItem[]>([]);
  const [brands, setBrands] = useState<LookupItem[]>([]);
  const [units, setUnits] = useState<Unit[]>([]);
  const [profiles, setProfiles] = useState<BusinessProfile[]>([]);
  const [query, setQuery] = useState("");
  const [categoryId, setCategoryId] = useState("");
  const [brandId, setBrandId] = useState("");
  const [showArchived, setShowArchived] = useState(false);
  const [editing, setEditing] = useState<{ id: string | null } | null>(null);
  const [stockFor, setStockFor] = useState<ProductListItem | null>(null);
  const [error, setError] = useState<string | null>(null);

  const loadRefs = useCallback(() => {
    Promise.all([api.listLookups("category"), api.listLookups("brand"), api.listUnits(), api.listBusinessProfiles()])
      .then(([c, b, u, p]) => {
        setCategories(c);
        setBrands(b);
        setUnits(u);
        setProfiles(p);
      })
      .catch((e) => setError(String(e)));
  }, []);
  useEffect(loadRefs, [loadRefs]);

  const loadProducts = useCallback(() => {
    api
      .listProducts({
        query: query.trim() || undefined,
        categoryId: categoryId || undefined,
        brandId: brandId || undefined,
        includeArchived: showArchived,
      })
      .then((r) => {
        setProducts(r);
        setError(null);
      })
      .catch((e) => setError(String(e)));
  }, [query, categoryId, brandId, showArchived]);

  // بحث فوري مع تأخير بسيط لتخفيف الاستعلامات أثناء الكتابة
  useEffect(() => {
    const t = setTimeout(loadProducts, 200);
    return () => clearTimeout(t);
  }, [loadProducts]);

  async function toggleArchive(p: ProductListItem) {
    try {
      await api.setProductArchived(p.id, !p.archived);
      loadProducts();
    } catch (e) {
      setError(String(e));
    }
  }

  const profile = profiles.find((p) => p.code === profileCode) ?? null;

  return (
    <div>
      <div className="flex items-start justify-between">
        <PageHeader title="المنتجات" description="ابحث بالاسم العربي أو الإنجليزي أو SKU أو الباركود." />
        <button className={btnPrimary} onClick={() => setEditing({ id: null })} disabled={units.length === 0}>
          <Plus size={16} /> منتج جديد
        </button>
      </div>

      <div className={`${cardClass} mb-4 flex flex-wrap items-center gap-3 p-3`}>
        <div className="relative min-w-64 flex-1">
          <Search size={16} className="pointer-events-none absolute start-3 top-2.5 text-slate-400" />
          <input
            className={`${inputClass} ps-9`}
            placeholder="بحث…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            aria-label="بحث في المنتجات"
          />
        </div>
        <select className={`${inputClass} w-44`} value={categoryId} onChange={(e) => setCategoryId(e.target.value)} aria-label="التصنيف">
          <option value="">كل التصنيفات</option>
          {categories.map((c) => <option key={c.id} value={c.id}>{c.nameAr}</option>)}
        </select>
        <select className={`${inputClass} w-44`} value={brandId} onChange={(e) => setBrandId(e.target.value)} aria-label="الماركة">
          <option value="">كل الماركات</option>
          {brands.map((b) => <option key={b.id} value={b.id}>{b.nameAr}</option>)}
        </select>
        <label className="flex items-center gap-2 text-sm">
          <input type="checkbox" checked={showArchived} onChange={(e) => setShowArchived(e.target.checked)} />
          عرض المؤرشف
        </label>
      </div>

      {error && <div className="mb-4"><Alert kind="error">{error}</Alert></div>}

      <div className={`${cardClass} overflow-x-auto`}>
        <table className="w-full text-sm">
          <thead className="bg-slate-50 text-slate-500 dark:bg-slate-900/40">
            <tr>
              <th className="px-4 py-2 text-start">المنتج</th>
              <th className="px-4 py-2 text-start">التصنيف / الماركة</th>
              <th className="px-4 py-2 text-start">الباركود</th>
              <th className="px-4 py-2 text-start">الوحدة</th>
              <th className="px-4 py-2 text-start">سعر الشراء</th>
              <th className="px-4 py-2 text-start">سعر البيع</th>
              <th className="px-4 py-2 text-start">الرصيد</th>
              <th className="px-4 py-2 text-start">الحالة</th>
              <th className="w-32" />
            </tr>
          </thead>
          <tbody>
            {products.length === 0 && (
              <tr><td colSpan={9} className="px-4 py-10 text-center text-slate-500">لا توجد منتجات مطابقة</td></tr>
            )}
            {products.map((p) => (
              <tr key={p.id} className={`border-t border-slate-100 dark:border-slate-700 ${p.archived ? "opacity-60" : ""}`}>
                <td className="px-4 py-2">
                  <div className="font-medium">{p.nameAr}</div>
                  <div className="text-xs text-slate-500" dir="ltr">
                    {[p.nameEn, p.sku].filter(Boolean).join(" · ")}
                    {p.variantCount > 0 && ` · ${p.variantCount} متغير`}
                  </div>
                </td>
                <td className="px-4 py-2">{[p.categoryName, p.brandName].filter(Boolean).join(" / ") || "—"}</td>
                <td className="px-4 py-2" dir="ltr">{p.firstBarcode ?? "—"}</td>
                <td className="px-4 py-2">{p.baseUnitName}</td>
                <td className="px-4 py-2">{formatMoney(p.purchasePrice)}</td>
                <td className="px-4 py-2 font-semibold">{formatMoney(p.salePrice)}</td>
                <td className={`px-4 py-2 ${p.stockQty <= 0 ? "text-red-600" : p.minStockQty > 0 && p.stockQty <= p.minStockQty ? "text-amber-600" : ""}`}>
                  {formatQty(p.stockQty)}
                </td>
                <td className="px-4 py-2">
                  {p.archived ? <span className="text-slate-500">مؤرشف</span>
                    : p.isActive ? <span className="text-teal-700">نشط</span>
                    : <span className="text-amber-600">غير نشط</span>}
                  {p.minStockQty > 0 && <div className="text-xs text-slate-500">حد الطلب: {formatQty(p.minStockQty)}</div>}
                </td>
                <td className="px-2">
                  <div className="flex gap-1">
                    <button className={btnGhost} aria-label="رصيد المخزون" onClick={() => setStockFor(p)}><Boxes size={14} /></button>
                    <button className={btnGhost} aria-label="تعديل" onClick={() => setEditing({ id: p.id })}><Pencil size={14} /></button>
                    <button className={btnGhost} aria-label={p.archived ? "استرجاع" : "أرشفة"} onClick={() => toggleArchive(p)}>
                      {p.archived ? <ArchiveRestore size={14} /> : <Archive size={14} />}
                    </button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {stockFor && (
        <StockDialog product={stockFor} onClose={() => setStockFor(null)} onDone={() => { setStockFor(null); loadProducts(); }} />
      )}

      {editing && (
        <Modal wide title={editing.id ? "تعديل منتج" : "منتج جديد"} onClose={() => setEditing(null)}>
          <ProductForm
            productId={editing.id}
            profile={profile}
            categories={categories}
            brands={brands}
            units={units}
            onCancel={() => setEditing(null)}
            onSaved={() => { setEditing(null); loadProducts(); loadRefs(); }}
          />
        </Modal>
      )}
    </div>
  );
}
