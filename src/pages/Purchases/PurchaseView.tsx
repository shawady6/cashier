import { useCallback, useEffect, useState } from "react";
import Alert from "../../components/Alert";
import Modal from "../../components/Modal";
import { btnGhost, btnPrimary, inputClass } from "../../components/ui";
import { formatMoney, formatQty } from "../../lib/format";
import { api, type PurchaseDetail } from "../../services/api";
import { statusLabel } from "./index";

export default function PurchaseView({ id, onClose, startConfirm = false }: { id: string; onClose: () => void; startConfirm?: boolean }) {
  const [d, setD] = useState<PurchaseDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [payAmount, setPayAmount] = useState("");
  const [payMethod, setPayMethod] = useState<"cash" | "card">("cash");
  const [retQty, setRetQty] = useState<Record<string, string>>({});
  const [showReturn, setShowReturn] = useState(false);
  const [confirming, setConfirming] = useState(startConfirm);

  const load = useCallback(() => {
    api.getPurchase(id).then(setD).catch((e) => setError(String(e)));
  }, [id]);
  useEffect(load, [load]);

  async function run(fn: () => Promise<unknown>, ok: string) {
    setError(null);
    setMsg(null);
    try {
      await fn();
      if (ok) setMsg(ok);
      load();
    } catch (e) {
      setError(String(e));
    }
  }

  if (!d) return <Modal title="فاتورة شراء" onClose={onClose}>{error ? <Alert kind="error">{error}</Alert> : "جارٍ التحميل..."}</Modal>;
  const received = d.header.status === "received";
  const isDraft = d.header.status === "draft";
  const priceChanges = d.items.filter((i) => i.newSalePriceMinor != null);

  return (
    <Modal title={`فاتورة شراء ${d.header.purchaseNo}`} onClose={onClose} wide>
      <div className="space-y-4">
        {error && <Alert kind="error">{error}</Alert>}
        {msg && <Alert kind="ok">{msg}</Alert>}
        <div className="grid gap-2 text-sm sm:grid-cols-4">
          <div>المورد: <b>{d.header.supplierName}</b></div>
          <div>المخزن: <b>{d.warehouseName}</b></div>
          <div>الحالة: <b>{statusLabel[d.header.status] ?? d.header.status}</b></div>
          <div>فاتورة المورد: <b>{d.header.supplierInvoiceNo || "—"}</b></div>
        </div>
        <table className="w-full text-sm">
          <thead className="text-slate-500"><tr><th className="p-2 text-start">الصنف</th><th className="p-2 text-start">الكمية</th><th className="p-2 text-start">التكلفة</th><th className="p-2 text-start">الإجمالي</th><th className="p-2 text-start">مرتجع</th>{showReturn && <th className="p-2 text-start">كمية المرتجع</th>}</tr></thead>
          <tbody>
            {d.items.map((i) => (
              <tr key={i.id} className="border-t border-slate-100 dark:border-slate-700">
                <td className="p-2">{i.nameAr}{i.variantName ? ` — ${i.variantName}` : ""}</td>
                <td className="p-2">
                  {formatQty(i.qtyMilli)} {i.unitName}
                  {i.unitFactorMicro !== 1_000_000 && <span className="block text-xs text-slate-500">= {formatQty(i.baseQtyMilli)} {i.baseUnitName}</span>}
                </td>
                <td className="p-2">{formatMoney(i.unitCostMinor)}</td>
                <td className="p-2">{formatMoney(i.netMinor)}</td>
                <td className="p-2">{i.returnedQtyMilli ? formatQty(i.returnedQtyMilli) : ""}</td>
                {showReturn && (
                  <td className="p-2 w-28"><input className={inputClass} dir="ltr" inputMode="decimal" value={retQty[i.id] ?? ""} onChange={(e) => setRetQty((s) => ({ ...s, [i.id]: e.target.value }))} /></td>
                )}
              </tr>
            ))}
          </tbody>
        </table>
        <div className="grid gap-1 text-sm sm:w-72">
          <div className="flex justify-between"><span>المجموع</span><span>{formatMoney(d.subtotalMinor)}</span></div>
          <div className="flex justify-between"><span>الخصم</span><span>{formatMoney(d.discountMinor)}</span></div>
          <div className="flex justify-between font-bold"><span>الإجمالي</span><span>{formatMoney(d.header.totalMinor)}</span></div>
          {received && <>
            <div className="flex justify-between"><span>المرتجع</span><span>{formatMoney(d.header.returnedMinor)}</span></div>
            <div className="flex justify-between"><span>المدفوع</span><span>{formatMoney(d.header.paidMinor)}</span></div>
            <div className="flex justify-between font-bold"><span>المتبقي للمورد</span><span>{formatMoney(d.outstandingMinor)}</span></div>
          </>}
        </div>

        {d.payments.length > 0 && (
          <div className="text-sm"><h4 className="mb-1 font-semibold">الدفعات</h4>
            {d.payments.map((p) => <div key={p.id}>{p.method === "cash" ? "نقدي" : "بطاقة"} — {formatMoney(p.amountMinor)} <span className="text-slate-400" dir="ltr">{p.createdAt.slice(0, 16).replace("T", " ")}</span></div>)}
          </div>
        )}
        {d.returns.length > 0 && (
          <div className="text-sm"><h4 className="mb-1 font-semibold">المرتجعات</h4>
            {d.returns.map((r) => <div key={r.id}><span dir="ltr">{r.returnNo}</span> — {formatMoney(r.totalMinor)}</div>)}
          </div>
        )}

        {received && (
          <div className="flex flex-wrap items-end gap-3 border-t border-slate-200 pt-3 dark:border-slate-700">
            {d.outstandingMinor > 0 && (
              <div className="flex items-end gap-2">
                <label className="text-sm">دفعة للمورد
                  <input className={`${inputClass} mt-1 w-32`} dir="ltr" inputMode="decimal" value={payAmount} onChange={(e) => setPayAmount(e.target.value)} />
                </label>
                <select className={`${inputClass} w-28`} value={payMethod} onChange={(e) => setPayMethod(e.target.value as "cash" | "card")}>
                  <option value="cash">نقدي</option><option value="card">بطاقة</option>
                </select>
                <button className={btnPrimary} disabled={!payAmount.trim()} onClick={() => run(async () => { await api.addSupplierPayment({ purchaseId: d.header.id, method: payMethod, amount: payAmount }); setPayAmount(""); }, "تم تسجيل الدفعة")}>تسجيل</button>
              </div>
            )}
            {!showReturn
              ? <button className={btnGhost} onClick={() => setShowReturn(true)}>مرتجع للمورد</button>
              : <button className={btnPrimary} onClick={() => run(async () => {
                  const items = Object.entries(retQty).filter(([, q]) => q.trim()).map(([purchaseItemId, qty]) => ({ purchaseItemId, qty }));
                  const r = await api.createPurchaseReturn({ purchaseId: d.header.id, items });
                  setRetQty({}); setShowReturn(false);
                  setMsg(`تم المرتجع ${r.returnNo} بقيمة ${formatMoney(r.credit)}`);
                }, "")}>تنفيذ المرتجع</button>}
          </div>
        )}

        {isDraft && !confirming && (
          <div className="flex justify-end gap-2">
            <button className={btnGhost} onClick={() => run(async () => { await api.cancelPurchaseDraft(d.header.id); onClose(); }, "تم الإلغاء")}>إلغاء المسودة</button>
            <button className={btnPrimary} onClick={() => setConfirming(true)}>استلام</button>
          </div>
        )}
        {isDraft && confirming && (
          <div className="space-y-3 rounded-lg border border-teal-300 bg-teal-50 p-3 text-sm dark:border-teal-700 dark:bg-teal-950/30">
            <h4 className="font-semibold">تأكيد الاستلام</h4>
            <div>سيُضاف إلى {d.warehouseName}: <b>{d.items.length}</b> صنف بإجمالي <b>{formatMoney(d.header.totalMinor)}</b>، وتتحدّث التكلفة المتوسطة.</div>
            {priceChanges.length > 0 && (
              <div>
                <div className="font-semibold">أسعار البيع التي ستتحدّث:</div>
                <ul className="list-inside list-disc">
                  {priceChanges.map((i) => <li key={i.id}>{i.nameAr}{i.variantName ? ` — ${i.variantName}` : ""}: {i.unitName} ← {formatMoney(i.newSalePriceMinor as number)}</li>)}
                </ul>
              </div>
            )}
            <div className="flex justify-end gap-2">
              <button className={btnGhost} onClick={() => setConfirming(false)}>رجوع</button>
              <button className={btnPrimary} onClick={() => run(async () => { await api.receivePurchase(d.header.id); setConfirming(false); }, "تم استلام الفاتورة وتحديث المخزن")}>تأكيد الاستلام</button>
            </div>
          </div>
        )}
      </div>
    </Modal>
  );
}
