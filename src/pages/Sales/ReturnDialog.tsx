import { useState } from "react";
import Alert from "../../components/Alert";
import Modal from "../../components/Modal";
import { btnGhost, btnPrimary, inputClass } from "../../components/ui";
import { formatQty } from "../../lib/format";
import { milliToQty } from "../../lib/qty";
import { api, type SaleDetail } from "../../services/api";

/** المرتجع يُنشئ سجلات عكسية جديدة ولا يعدّل الفاتورة. المبلغ المسترد يحسبه Rust. */
export default function ReturnDialog({ sale, onDone, onClose }: { sale: SaleDetail; onDone: (refund: number, no: string) => void; onClose: () => void }) {
  const [qtys, setQtys] = useState<Record<string, string>>({});
  const [method, setMethod] = useState<"cash" | "card">("cash");
  const [reason, setReason] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit() {
    const items = Object.entries(qtys).filter(([, q]) => q.trim() !== "" && q.trim() !== "0").map(([saleItemId, qty]) => ({ saleItemId, qty }));
    if (items.length === 0) return setError("حدّد كمية بند واحد على الأقل");
    setBusy(true);
    setError(null);
    try {
      const r = await api.createSalesReturn({ saleId: sale.id, items, reason, refundMethod: method });
      onDone(r.refund, r.returnNo);
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  }

  return (
    <Modal wide title={`مرتجع على الفاتورة ${sale.invoiceNo}`} onClose={onClose}>
      <div className="space-y-4">
        {error && <Alert kind="error">{error}</Alert>}
        <table className="w-full text-sm">
          <thead className="text-slate-500"><tr>
            <th className="py-2 text-start">البند</th><th className="text-start">المباع</th><th className="text-start">سبق إرجاعه</th><th className="text-start">كمية المرتجع</th>
          </tr></thead>
          <tbody>
            {sale.items.map((i) => {
              const left = i.qtyMilli - i.returnedQtyMilli;
              return (
                <tr key={i.id} className="border-t border-slate-100 dark:border-slate-700">
                  <td className="py-2">{i.nameAr}{i.variantName ? ` - ${i.variantName}` : ""} <span className="text-slate-500">({i.unitName})</span></td>
                  <td>{formatQty(i.qtyMilli)}</td>
                  <td>{formatQty(i.returnedQtyMilli)}</td>
                  <td>
                    {left > 0 ? (
                      <div className="flex items-center gap-2">
                        <input dir="ltr" inputMode="decimal" className={`${inputClass} w-24`} value={qtys[i.id] ?? ""} placeholder="0"
                          onChange={(e) => setQtys({ ...qtys, [i.id]: e.target.value })} />
                        <button type="button" className="text-teal-700 underline" onClick={() => setQtys({ ...qtys, [i.id]: milliToQty(left) })}>الكل</button>
                      </div>
                    ) : <span className="text-slate-400">تم إرجاعه بالكامل</span>}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
        <div className="grid gap-3 md:grid-cols-2">
          <label className="text-sm">طريقة الاسترداد
            <select className={`${inputClass} mt-1`} value={method} onChange={(e) => setMethod(e.target.value as "cash" | "card")}>
              <option value="cash">نقدًا</option><option value="card">كارت</option>
            </select>
          </label>
          <label className="text-sm">السبب (اختياري)
            <input className={`${inputClass} mt-1`} value={reason} onChange={(e) => setReason(e.target.value)} />
          </label>
        </div>
        <div className="flex justify-end gap-2">
          <button className={btnGhost} onClick={onClose} disabled={busy}>إلغاء</button>
          <button className={btnPrimary} onClick={submit} disabled={busy}>تأكيد المرتجع</button>
        </div>
      </div>
    </Modal>
  );
}
