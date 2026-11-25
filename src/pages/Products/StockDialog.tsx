import { useEffect, useState } from "react";
import Alert from "../../components/Alert";
import Modal from "../../components/Modal";
import { btnGhost, btnPrimary, inputClass } from "../../components/ui";
import { formatQty } from "../../lib/format";
import { api, type ProductDetail, type ProductListItem } from "../../services/api";

/** رصيد افتتاحي وتسوية يدوية (زيادة بتكلفة، أو نقص بمتوسط التكلفة). التكلفة والرصيد يحسبهما Rust. */
export default function StockDialog({ product, onDone, onClose }: { product: ProductListItem; onDone: () => void; onClose: () => void }) {
  const [detail, setDetail] = useState<ProductDetail | null>(null);
  const [variantId, setVariantId] = useState("");
  const [direction, setDirection] = useState<"increase" | "decrease">("increase");
  const [qty, setQty] = useState("");
  const [cost, setCost] = useState("");
  const [note, setNote] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    api.getProduct(product.id).then((d) => {
      setDetail(d);
      const active = d.variants.filter((v) => v.isActive);
      if (active.length > 0) setVariantId(active[0].id);
    }).catch((e) => setError(String(e)));
  }, [product.id]);

  const needsVariant = (detail?.variants.filter((v) => v.isActive).length ?? 0) > 0;

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await api.adjustStock({
        productId: product.id,
        variantId: needsVariant ? variantId : null,
        direction,
        qty,
        unitCost: direction === "increase" && cost.trim() ? cost : undefined,
        note,
      });
      onDone();
    } catch (err) {
      setError(String(err));
      setBusy(false);
    }
  }

  return (
    <Modal title={`رصيد المخزون: ${product.nameAr}`} onClose={onClose}>
      <form onSubmit={submit} className="space-y-4">
        {error && <Alert kind="error">{error}</Alert>}
        <div className="text-sm text-slate-500">الرصيد الحالي: {formatQty(product.stockQty)} {product.baseUnitName}</div>
        {needsVariant && detail && (
          <label className="block text-sm">المتغير
            <select className={`${inputClass} mt-1`} value={variantId} onChange={(e) => setVariantId(e.target.value)}>
              {detail.variants.filter((v) => v.isActive).map((v) => <option key={v.id} value={v.id}>{v.nameAr}</option>)}
            </select>
          </label>
        )}
        <div className="flex gap-4 text-sm">
          <label className="flex items-center gap-2"><input type="radio" checked={direction === "increase"} onChange={() => setDirection("increase")} /> إضافة رصيد</label>
          <label className="flex items-center gap-2"><input type="radio" checked={direction === "decrease"} onChange={() => setDirection("decrease")} /> خصم رصيد (تالف/عجز)</label>
        </div>
        <div className="grid gap-3 md:grid-cols-2">
          <label className="block text-sm">الكمية ({product.baseUnitName})
            <input autoFocus inputMode="decimal" required className={`${inputClass} mt-1`} value={qty} onChange={(e) => setQty(e.target.value)} />
          </label>
          {direction === "increase" && (
            <label className="block text-sm">تكلفة الوحدة (جنيه)
              <input inputMode="decimal" className={`${inputClass} mt-1`} value={cost} placeholder="الافتراضي: سعر الشراء" onChange={(e) => setCost(e.target.value)} />
            </label>
          )}
        </div>
        <label className="block text-sm">ملاحظة
          <input className={`${inputClass} mt-1`} value={note} onChange={(e) => setNote(e.target.value)} />
        </label>
        <p className="text-xs text-slate-500">الاستلام من الموردين وفواتير الشراء تأتي في المرحلة 3.</p>
        <div className="flex justify-end gap-2">
          <button type="button" className={btnGhost} onClick={onClose}>إلغاء</button>
          <button type="submit" className={btnPrimary} disabled={busy}>حفظ</button>
        </div>
      </form>
    </Modal>
  );
}
