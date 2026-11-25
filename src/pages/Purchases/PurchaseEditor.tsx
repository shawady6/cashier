import { Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import Alert from "../../components/Alert";
import ItemPicker from "../../components/ItemPicker";
import Modal from "../../components/Modal";
import { btnGhost, btnPrimary, inputClass } from "../../components/ui";
import { microToInput, milliToInput, minorToInput } from "../../lib/format";
import { api, type SellUnit, type Supplier, type Warehouse } from "../../services/api";

interface Line {
  productId: string;
  variantId: string | null;
  name: string;
  unitId: string | null;
  baseUnitId: string;
  baseUnitName: string;
  units: SellUnit[];
  qty: string;
  unitCost: string;
}

/** مسودة فاتورة شراء. الحسابات (الإجماليات والخصم) في Rust عند الحفظ والعرض. */
export default function PurchaseEditor({ id, onClose, onSaved }: { id: string | null; onClose: () => void; onSaved: () => void }) {
  const [suppliers, setSuppliers] = useState<Supplier[]>([]);
  const [warehouses, setWarehouses] = useState<Warehouse[]>([]);
  const [supplierId, setSupplierId] = useState("");
  const [warehouseId, setWarehouseId] = useState("");
  const [invoiceNo, setInvoiceNo] = useState("");
  const [note, setNote] = useState("");
  const [discKind, setDiscKind] = useState<"amount" | "percent">("amount");
  const [discValue, setDiscValue] = useState("");
  const [lines, setLines] = useState<Line[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const [s, w] = await Promise.all([api.listSuppliers(), api.listWarehouses()]);
        const as = s.filter((x) => x.isActive);
        const aw = w.filter((x) => x.isActive);
        setSuppliers(as);
        setWarehouses(aw);
        setSupplierId(as[0]?.id ?? "");
        setWarehouseId(aw.find((x) => x.isDefault)?.id ?? aw[0]?.id ?? "");
        if (id) {
          const d = await api.getPurchase(id);
          setSupplierId(d.supplierId);
          setWarehouseId(d.warehouseId);
          setInvoiceNo(d.header.supplierInvoiceNo);
          setNote(d.note);
          if (d.discountMinor > 0) setDiscValue(minorToInput(d.discountMinor));
          setLines(
            d.items.map((i) => ({
              productId: i.productId, variantId: i.variantId, name: i.nameAr + (i.variantName ? ` — ${i.variantName}` : ""),
              unitId: i.unitId, baseUnitId: i.unitId, baseUnitName: i.unitName, units: [],
              qty: milliToInput(i.qtyMilli), unitCost: minorToInput(i.unitCostMinor),
            })),
          );
        }
      } catch (e) {
        setError(String(e));
      }
    })();
  }, [id]);

  async function save() {
    setBusy(true);
    setError(null);
    try {
      await api.savePurchaseDraft({
        id: id ?? undefined,
        supplierId,
        warehouseId,
        supplierInvoiceNo: invoiceNo,
        note,
        discount: discValue.trim() ? { kind: discKind, value: discValue } : null,
        items: lines.map((l) => ({
          productId: l.productId, variantId: l.variantId,
          unitId: l.unitId === l.baseUnitId ? null : l.unitId, qty: l.qty, unitCost: l.unitCost,
        })),
      });
      onSaved();
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  }

  const upd = (i: number, patch: Partial<Line>) => setLines((ls) => ls.map((x, j) => (j === i ? { ...x, ...patch } : x)));

  return (
    <Modal title={id ? "تعديل مسودة شراء" : "فاتورة شراء جديدة"} onClose={onClose} wide>
      <div className="space-y-3">
        {error && <Alert kind="error">{error}</Alert>}
        <div className="grid gap-3 sm:grid-cols-3">
          <label className="text-sm">المورد
            <select className={`${inputClass} mt-1`} value={supplierId} onChange={(e) => setSupplierId(e.target.value)}>
              {suppliers.length === 0 && <option value="">أضف موردًا أولًا</option>}
              {suppliers.map((s) => <option key={s.id} value={s.id}>{s.nameAr}</option>)}
            </select>
          </label>
          <label className="text-sm">المخزن
            <select className={`${inputClass} mt-1`} value={warehouseId} onChange={(e) => setWarehouseId(e.target.value)}>
              {warehouses.map((w) => <option key={w.id} value={w.id}>{w.nameAr}</option>)}
            </select>
          </label>
          <label className="text-sm">رقم فاتورة المورد
            <input className={`${inputClass} mt-1`} value={invoiceNo} onChange={(e) => setInvoiceNo(e.target.value)} />
          </label>
        </div>
        <ItemPicker
          onPick={(s) =>
            setLines((ls) => [...ls, {
              productId: s.productId, variantId: s.variantId, name: s.nameAr + (s.variantName ? ` — ${s.variantName}` : ""),
              unitId: s.baseUnitId, baseUnitId: s.baseUnitId, baseUnitName: s.baseUnitName, units: s.units,
              qty: "1", unitCost: "",
            }])
          }
        />
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="text-slate-500"><tr><th className="p-2 text-start">الصنف</th><th className="p-2 text-start">الوحدة</th><th className="p-2 text-start">الكمية</th><th className="p-2 text-start">تكلفة الوحدة (ج.م)</th><th /></tr></thead>
            <tbody>
              {lines.length === 0 && <tr><td colSpan={5} className="p-4 text-center text-slate-400">ابحث وأضف أصنافًا.</td></tr>}
              {lines.map((l, i) => (
                <tr key={i} className="border-t border-slate-100 dark:border-slate-700">
                  <td className="p-2">{l.name}</td>
                  <td className="p-2">
                    {l.units.length > 1 ? (
                      <select className={inputClass} value={l.unitId ?? ""} onChange={(e) => upd(i, { unitId: e.target.value })}>
                        {l.units.map((u) => <option key={u.unitId} value={u.unitId}>{u.nameAr}{u.factorMicro !== 1_000_000 ? ` (×${microToInput(u.factorMicro)})` : ""}</option>)}
                      </select>
                    ) : l.baseUnitName}
                  </td>
                  <td className="p-2 w-28"><input className={inputClass} dir="ltr" inputMode="decimal" value={l.qty} onChange={(e) => upd(i, { qty: e.target.value })} /></td>
                  <td className="p-2 w-36"><input className={inputClass} dir="ltr" inputMode="decimal" value={l.unitCost} onChange={(e) => upd(i, { unitCost: e.target.value })} /></td>
                  <td className="p-2"><button aria-label="حذف" onClick={() => setLines((ls) => ls.filter((_, j) => j !== i))}><Trash2 size={16} /></button></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="grid gap-3 sm:grid-cols-3">
          <label className="text-sm">خصم الفاتورة
            <div className="mt-1 flex gap-2">
              <input className={inputClass} dir="ltr" inputMode="decimal" value={discValue} onChange={(e) => setDiscValue(e.target.value)} />
              <select className={`${inputClass} w-24`} value={discKind} onChange={(e) => setDiscKind(e.target.value as "amount" | "percent")}>
                <option value="amount">ج.م</option><option value="percent">%</option>
              </select>
            </div>
          </label>
          <label className="text-sm sm:col-span-2">ملاحظة
            <input className={`${inputClass} mt-1`} value={note} onChange={(e) => setNote(e.target.value)} />
          </label>
        </div>
        <div className="flex justify-end gap-2">
          <button className={btnGhost} onClick={onClose}>إلغاء</button>
          <button className={btnPrimary} disabled={busy || !supplierId || lines.length === 0} onClick={save}>حفظ كمسودة</button>
        </div>
      </div>
    </Modal>
  );
}
