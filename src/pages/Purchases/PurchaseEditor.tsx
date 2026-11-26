import { Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import Alert from "../../components/Alert";
import ItemPicker from "../../components/ItemPicker";
import Modal from "../../components/Modal";
import { btnGhost, btnPrimary, inputClass } from "../../components/ui";
import { buildChoices, lineForChoice, type UnitChoice, type UnitLineState } from "../../lib/bigUnit";
import { minorToInput, milliToInput } from "../../lib/format";
import UnitLineFields from "../../components/UnitLineFields";
import { api, type ProductDetail, type Sellable, type Supplier, type Unit, type Warehouse } from "../../services/api";

interface Line {
  key: string;
  productId: string;
  variantId: string | null;
  name: string;
  choices: UnitChoice[];
  state: UnitLineState;
}

let lineCounter = 0;

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
  const [units, setUnits] = useState<Unit[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const [s, w, allUnits] = await Promise.all([api.listSuppliers(), api.listWarehouses(), api.listUnits()]);
        setUnits(allUnits);
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
          const cache = new Map<string, ProductDetail>();
          const built: Line[] = [];
          for (const i of d.items) {
            let det = cache.get(i.productId);
            if (!det) {
              det = await api.getProduct(i.productId);
              cache.set(i.productId, det);
            }
            const choices = buildChoices(det, allUnits, i.variantId, true);
            const c = choices.find((x) => x.unitId === i.unitId) ?? choices[0];
            const base = lineForChoice(c, milliToInput(i.qtyMilli));
            built.push({
              key: `l${lineCounter++}`, productId: i.productId, variantId: i.variantId,
              name: i.nameAr + (i.variantName ? ` — ${i.variantName}` : ""), choices,
              state: {
                ...base,
                factor: c.isBase ? "1" : String(i.unitFactorMicro / 1_000_000),
                cost: minorToInput(i.unitCostMinor),
                salePrice: i.newSalePriceMinor != null ? minorToInput(i.newSalePriceMinor) : base.salePrice,
              },
            });
          }
          setLines(built);
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
        items: lines.map((l) => {
          const c = l.choices.find((x) => x.unitId === l.state.unitId) ?? l.choices[0];
          const price = l.state.salePrice.trim();
          const changed = price !== "" && price !== l.state.origSalePrice;
          return {
            productId: l.productId,
            variantId: l.variantId,
            unitId: c.isBase ? null : c.unitId,
            qty: l.state.qty,
            unitCost: l.state.cost,
            factor: c.isBase ? undefined : l.state.factor,
            salePrice: changed ? price : undefined,
          };
        }),
      });
      onSaved();
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  }

  async function addLine(sel: Sellable) {
    try {
      const det = await api.getProduct(sel.productId);
      const choices = buildChoices(det, units, sel.variantId, true);
      setLines((ls) => [...ls, {
        key: `l${lineCounter++}`, productId: sel.productId, variantId: sel.variantId,
        name: sel.nameAr + (sel.variantName ? ` — ${sel.variantName}` : ""), choices, state: lineForChoice(choices[0]),
      }]);
    } catch (e) {
      setError(String(e));
    }
  }

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
        <ItemPicker onPick={(sel) => void addLine(sel)} />
        <div className="space-y-3">
          {lines.length === 0 && <p className="p-4 text-center text-sm text-slate-400">ابحث وأضف أصنافًا.</p>}
          {lines.map((l) => (
            <div key={l.key} className="rounded-lg border border-slate-200 p-3 dark:border-slate-700">
              <div className="mb-2 flex items-center justify-between text-sm font-semibold">
                <span>{l.name}</span>
                <button aria-label="حذف" onClick={() => setLines((ls) => ls.filter((x) => x.key !== l.key))}><Trash2 size={16} /></button>
              </div>
              <UnitLineFields
                choices={l.choices}
                value={l.state}
                isVariant={l.variantId !== null}
                onChange={(st) => setLines((ls) => ls.map((x) => (x.key === l.key ? { ...x, state: st } : x)))}
              />
            </div>
          ))}
        </div>
        <p className="text-xs text-slate-500">الفاتورة تُحفظ دائمًا كمسودة. المخزون والتكلفة وأسعار البيع لا تتغير إلا عند الضغط على «استلام».</p>
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
