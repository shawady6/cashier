import { Plus, Trash2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import Alert from "../../components/Alert";
import ItemPicker from "../../components/ItemPicker";
import Modal from "../../components/Modal";
import { btnGhost, btnPrimary, cardClass, inputClass } from "../../components/ui";
import { formatMoney } from "../../lib/format";
import { api, type TransferRow, type Warehouse } from "../../services/api";

interface Line { productId: string; variantId: string | null; name: string; unit: string; qty: string }

export default function TransfersTab() {
  const [rows, setRows] = useState<TransferRow[]>([]);
  const [wh, setWh] = useState<Warehouse[]>([]);
  const [open, setOpen] = useState(false);
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [note, setNote] = useState("");
  const [lines, setLines] = useState<Line[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(() => {
    api.listTransfers().then(setRows).catch((e) => setError(String(e)));
    api.listWarehouses().then((w) => setWh(w.filter((x) => x.isActive))).catch(() => undefined);
  }, []);
  useEffect(load, [load]);

  async function submit() {
    setBusy(true);
    setError(null);
    try {
      await api.createTransfer({
        fromWarehouseId: from,
        toWarehouseId: to,
        note,
        items: lines.map((l) => ({ productId: l.productId, variantId: l.variantId, qty: l.qty })),
      });
      setOpen(false);
      setLines([]);
      setNote("");
      load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-3">
      {error && !open && <Alert kind="error">{error}</Alert>}
      <button className={btnPrimary} onClick={() => { setError(null); setFrom(wh[0]?.id ?? ""); setTo(wh[1]?.id ?? ""); setOpen(true); }}>
        <Plus size={16} /> تحويل جديد
      </button>
      <div className={`${cardClass} overflow-x-auto`}>
        <table className="w-full text-sm">
          <thead className="text-slate-500">
            <tr>
              <th className="p-3 text-start">الرقم</th><th className="p-3 text-start">من</th><th className="p-3 text-start">إلى</th>
              <th className="p-3 text-start">الأصناف</th><th className="p-3 text-start">القيمة</th><th className="p-3 text-start">التاريخ</th>
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 && <tr><td colSpan={6} className="p-6 text-center text-slate-400">لا توجد تحويلات.</td></tr>}
            {rows.map((t) => (
              <tr key={t.id} className="border-t border-slate-100 dark:border-slate-700">
                <td className="p-3" dir="ltr">{t.transferNo}</td><td className="p-3">{t.fromName}</td><td className="p-3">{t.toName}</td>
                <td className="p-3">{t.itemCount}</td><td className="p-3">{formatMoney(t.valueMinor)}</td>
                <td className="p-3" dir="ltr">{t.createdAt.slice(0, 16).replace("T", " ")}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {open && (
        <Modal title="تحويل بين المخازن" onClose={() => setOpen(false)} wide>
          <div className="space-y-3">
            {error && <Alert kind="error">{error}</Alert>}
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="text-sm">من مخزن
                <select className={`${inputClass} mt-1`} value={from} onChange={(e) => setFrom(e.target.value)}>
                  {wh.map((w) => <option key={w.id} value={w.id}>{w.nameAr}</option>)}
                </select>
              </label>
              <label className="text-sm">إلى مخزن
                <select className={`${inputClass} mt-1`} value={to} onChange={(e) => setTo(e.target.value)}>
                  {wh.map((w) => <option key={w.id} value={w.id}>{w.nameAr}</option>)}
                </select>
              </label>
            </div>
            <ItemPicker onPick={(s) => setLines((ls) => [...ls, { productId: s.productId, variantId: s.variantId, name: s.nameAr + (s.variantName ? ` — ${s.variantName}` : ""), unit: s.baseUnitName, qty: "1" }])} />
            <table className="w-full text-sm">
              <tbody>
                {lines.map((l, i) => (
                  <tr key={i} className="border-t border-slate-100 dark:border-slate-700">
                    <td className="p-2">{l.name}</td>
                    <td className="p-2 w-32">
                      <input className={inputClass} dir="ltr" inputMode="decimal" value={l.qty} onChange={(e) => setLines((ls) => ls.map((x, j) => (j === i ? { ...x, qty: e.target.value } : x)))} />
                    </td>
                    <td className="p-2 text-slate-500">{l.unit}</td>
                    <td className="p-2"><button aria-label="حذف" onClick={() => setLines((ls) => ls.filter((_, j) => j !== i))}><Trash2 size={16} /></button></td>
                  </tr>
                ))}
              </tbody>
            </table>
            <input className={inputClass} placeholder="ملاحظة (اختياري)" value={note} onChange={(e) => setNote(e.target.value)} />
            <div className="flex justify-end gap-2">
              <button className={btnGhost} onClick={() => setOpen(false)}>إلغاء</button>
              <button className={btnPrimary} disabled={busy || lines.length === 0} onClick={submit}>تنفيذ التحويل</button>
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
}
