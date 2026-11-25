import { Plus } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import Alert from "../../components/Alert";
import Modal from "../../components/Modal";
import { btnGhost, btnPrimary, cardClass, inputClass } from "../../components/ui";
import { formatQty, milliToInput } from "../../lib/format";
import { api, type CountDetail, type CountRow, type Warehouse } from "../../services/api";

const statusLabel: Record<string, string> = { open: "مفتوح", finalized: "معتمد", cancelled: "ملغي" };

export default function CountsTab() {
  const [rows, setRows] = useState<CountRow[]>([]);
  const [wh, setWh] = useState<Warehouse[]>([]);
  const [newWh, setNewWh] = useState<string | null>(null);
  const [detail, setDetail] = useState<CountDetail | null>(null);
  const [vals, setVals] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);

  const load = useCallback(() => {
    api.listCounts().then(setRows).catch((e) => setError(String(e)));
    api.listWarehouses().then((w) => setWh(w.filter((x) => x.isActive))).catch(() => undefined);
  }, []);
  useEffect(load, [load]);

  async function openCount(id: string) {
    setError(null);
    try {
      const d = await api.getCount(id);
      setDetail(d);
      setVals(Object.fromEntries(d.items.map((i) => [i.id, i.countedQtyMilli == null ? "" : milliToInput(i.countedQtyMilli)])));
    } catch (e) {
      setError(String(e));
    }
  }
  const lines = () => Object.entries(vals).map(([itemId, v]) => ({ itemId, countedQty: v.trim() === "" ? null : v }));

  async function create() {
    if (!newWh) return;
    try {
      const id = await api.createCount({ warehouseId: newWh });
      setNewWh(null);
      load();
      await openCount(id);
    } catch (e) {
      setError(String(e));
    }
  }
  async function run(fn: () => Promise<unknown>, okMsg?: string, close = false) {
    setError(null);
    try {
      await fn();
      if (okMsg) setMsg(okMsg);
      if (close) setDetail(null);
      load();
    } catch (e) {
      setError(String(e));
    }
  }

  const editable = detail?.header.status === "open";

  return (
    <div className="space-y-3">
      {error && !detail && <Alert kind="error">{error}</Alert>}
      {msg && <Alert kind="ok">{msg}</Alert>}
      <button className={btnPrimary} onClick={() => { setMsg(null); setNewWh(wh[0]?.id ?? ""); }}><Plus size={16} /> جرد جديد</button>
      <div className={`${cardClass} overflow-x-auto`}>
        <table className="w-full text-sm">
          <thead className="text-slate-500">
            <tr><th className="p-3 text-start">الرقم</th><th className="p-3 text-start">المخزن</th><th className="p-3 text-start">الحالة</th><th className="p-3 text-start">الأصناف</th><th className="p-3 text-start">التاريخ</th><th /></tr>
          </thead>
          <tbody>
            {rows.length === 0 && <tr><td colSpan={6} className="p-6 text-center text-slate-400">لا توجد عمليات جرد.</td></tr>}
            {rows.map((c) => (
              <tr key={c.id} className="border-t border-slate-100 dark:border-slate-700">
                <td className="p-3" dir="ltr">{c.countNo}</td><td className="p-3">{c.warehouseName}</td>
                <td className="p-3">{statusLabel[c.status] ?? c.status}</td><td className="p-3">{c.itemCount}</td>
                <td className="p-3" dir="ltr">{c.createdAt.slice(0, 16).replace("T", " ")}</td>
                <td className="p-3 text-end"><button className={btnGhost} onClick={() => openCount(c.id)}>{c.status === "open" ? "متابعة" : "عرض"}</button></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {newWh !== null && (
        <Modal title="جرد جديد" onClose={() => setNewWh(null)}>
          <div className="space-y-3">
            <label className="block text-sm">المخزن
              <select className={`${inputClass} mt-1`} value={newWh} onChange={(e) => setNewWh(e.target.value)}>
                {wh.map((w) => <option key={w.id} value={w.id}>{w.nameAr}</option>)}
              </select>
            </label>
            <p className="text-xs text-slate-500">يُسجَّل رصيد النظام لحظة بدء الجرد، والفرق يُطبَّق على الرصيد الحالي عند الاعتماد.</p>
            <div className="flex justify-end gap-2">
              <button className={btnGhost} onClick={() => setNewWh(null)}>إلغاء</button>
              <button className={btnPrimary} onClick={create}>بدء الجرد</button>
            </div>
          </div>
        </Modal>
      )}
      {detail && (
        <Modal title={`جرد ${detail.header.countNo} — ${detail.header.warehouseName}`} onClose={() => setDetail(null)} wide>
          <div className="space-y-3">
            {error && <Alert kind="error">{error}</Alert>}
            <div className="max-h-[55vh] overflow-y-auto">
              <table className="w-full text-sm">
                <thead className="text-slate-500"><tr><th className="p-2 text-start">الصنف</th><th className="p-2 text-start">رصيد النظام</th><th className="p-2 text-start">المعدود</th><th className="p-2 text-start">الفرق</th></tr></thead>
                <tbody>
                  {detail.items.map((i) => {
                    const v = vals[i.id] ?? "";
                    const counted = /^\d+(\.\d{1,3})?$/.test(v.trim()) ? Math.round(Number(v) * 1000) : null;
                    return (
                      <tr key={i.id} className="border-t border-slate-100 dark:border-slate-700">
                        <td className="p-2">{i.nameAr}{i.variantName ? ` — ${i.variantName}` : ""}</td>
                        <td className="p-2">{formatQty(i.systemQtyMilli)} {i.baseUnitName}</td>
                        <td className="p-2 w-32">
                          <input className={inputClass} dir="ltr" inputMode="decimal" disabled={!editable} value={v} onChange={(e) => setVals((s) => ({ ...s, [i.id]: e.target.value }))} />
                        </td>
                        <td className="p-2" dir="ltr">{counted == null ? "" : formatQty(counted - i.systemQtyMilli)}</td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
            {editable && (
              <div className="flex flex-wrap justify-end gap-2">
                <button className={btnGhost} onClick={() => run(() => api.cancelCount(detail.header.id), "تم إلغاء الجرد", true)}>إلغاء الجرد</button>
                <button className={btnGhost} onClick={() => run(() => api.saveCountLines(detail.header.id, lines()), "تم حفظ الأرقام")}>حفظ</button>
                <button className={btnPrimary} onClick={() => run(async () => { await api.saveCountLines(detail.header.id, lines()); await api.finalizeCount(detail.header.id); }, "تم اعتماد الجرد وتطبيق الفروق", true)}>اعتماد الجرد</button>
              </div>
            )}
          </div>
        </Modal>
      )}
    </div>
  );
}
