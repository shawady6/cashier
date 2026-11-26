import { Plus } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import Alert from "../../components/Alert";
import PageHeader from "../../components/PageHeader";
import { btnGhost, btnPrimary, cardClass, inputClass } from "../../components/ui";
import { formatMoney } from "../../lib/format";
import { api, type PurchaseRow } from "../../services/api";
import PurchaseEditor from "./PurchaseEditor";
import PurchaseView from "./PurchaseView";

export const statusLabel: Record<string, string> = { draft: "مسودة", received: "مستلمة", cancelled: "ملغاة" };

export default function PurchasesPage() {
  const [rows, setRows] = useState<PurchaseRow[]>([]);
  const [status, setStatus] = useState("");
  const [query, setQuery] = useState("");
  const [editing, setEditing] = useState<string | null | undefined>(undefined); // undefined = مغلق، null = جديد
  const [viewing, setViewing] = useState<{ id: string; confirm: boolean } | null>(null);
  const [pending, setPending] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    api.listPurchases({ status: status || undefined, query: query || undefined }).then(setRows).catch((e) => setError(String(e)));
    api.listPurchases({ status: "draft" }).then((r) => setPending(r.length)).catch(() => setPending(0));
  }, [status, query]);
  useEffect(load, [load]);

  return (
    <div className="max-w-6xl">
      <PageHeader title="المشتريات" />
      {error && <Alert kind="error">{error}</Alert>}
      {pending > 0 && (
        <div className="mb-3 flex items-center justify-between rounded-lg border border-amber-300 bg-amber-50 p-3 text-sm text-amber-900 dark:border-amber-700 dark:bg-amber-950/40 dark:text-amber-200">
          <span>عندك {pending} {pending === 1 ? "فاتورة شراء مسودة" : "فواتير شراء مسودة"} لم تُضف للمخزن</span>
          <button className={btnGhost} onClick={() => setStatus("draft")}>عرض المسودات</button>
        </div>
      )}
      <div className="mb-3 flex flex-wrap gap-2">
        <button className={btnPrimary} onClick={() => setEditing(null)}><Plus size={16} /> فاتورة شراء جديدة</button>
        <select className={`${inputClass} max-w-[10rem]`} value={status} onChange={(e) => setStatus(e.target.value)}>
          <option value="">كل الحالات</option>
          {Object.entries(statusLabel).map(([k, v]) => <option key={k} value={k}>{v}</option>)}
        </select>
        <input className={`${inputClass} max-w-xs`} placeholder="بحث برقم الفاتورة أو المورد" value={query} onChange={(e) => setQuery(e.target.value)} />
      </div>
      <div className={`${cardClass} overflow-x-auto`}>
        <table className="w-full text-sm">
          <thead className="text-slate-500">
            <tr>
              <th className="p-3 text-start">الرقم</th><th className="p-3 text-start">المورد</th><th className="p-3 text-start">فاتورة المورد</th>
              <th className="p-3 text-start">الحالة</th><th className="p-3 text-start">الإجمالي</th><th className="p-3 text-start">المدفوع</th><th className="p-3 text-start">المرتجع</th><th />
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 && <tr><td colSpan={8} className="p-6 text-center text-slate-400">لا توجد فواتير شراء.</td></tr>}
            {rows.map((p) => (
              <tr key={p.id} className="border-t border-slate-100 dark:border-slate-700">
                <td className="p-3" dir="ltr">{p.purchaseNo}</td><td className="p-3">{p.supplierName}</td><td className="p-3">{p.supplierInvoiceNo}</td>
                <td className="p-3">
                  {p.status === "draft"
                    ? <span className="rounded bg-amber-100 px-2 py-0.5 text-xs font-semibold text-amber-800 dark:bg-amber-900/40 dark:text-amber-200">مسودة — لم تُضف للمخزن</span>
                    : statusLabel[p.status] ?? p.status}
                </td><td className="p-3">{formatMoney(p.totalMinor)}</td>
                <td className="p-3">{formatMoney(p.paidMinor)}</td><td className="p-3">{formatMoney(p.returnedMinor)}</td>
                <td className="p-3 text-end">
                  {p.status === "draft" ? (
                    <>
                      <button className={btnGhost} onClick={() => setEditing(p.id)}>تعديل</button>
                      <button className={btnPrimary} onClick={() => setViewing({ id: p.id, confirm: true })}>استلام</button>
                    </>
                  ) : <button className={btnGhost} onClick={() => setViewing({ id: p.id, confirm: false })}>عرض</button>}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {editing !== undefined && (
        <PurchaseEditor id={editing} onClose={() => setEditing(undefined)} onSaved={(id, receiveNow) => { setEditing(undefined); load(); if (receiveNow) setViewing({ id, confirm: true }); }} />
      )}
      {viewing && <PurchaseView id={viewing.id} startConfirm={viewing.confirm} onClose={() => { setViewing(null); load(); }} />}
    </div>
  );
}
