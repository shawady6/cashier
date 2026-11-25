import { Eye } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import Alert from "../../components/Alert";
import Modal from "../../components/Modal";
import PageHeader from "../../components/PageHeader";
import ReceiptDialog from "../../components/ReceiptDialog";
import { btnGhost, btnPrimary, cardClass, inputClass } from "../../components/ui";
import { formatMoney, formatQty } from "../../lib/format";
import { api, type SaleDetail, type SaleListItem } from "../../services/api";
import ReturnDialog from "./ReturnDialog";

const methodLabel: Record<string, string> = { cash: "نقدًا", card: "كارت" };

export default function SalesPage() {
  const [rows, setRows] = useState<SaleListItem[]>([]);
  const [query, setQuery] = useState("");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [detail, setDetail] = useState<SaleDetail | null>(null);
  const [returning, setReturning] = useState(false);
  const [receiptFor, setReceiptFor] = useState<string | null>(null);

  const load = useCallback(() => {
    api
      .listSales({ query: query || undefined, dateFrom: from || undefined, dateTo: to || undefined })
      .then((r) => { setRows(r); setError(null); })
      .catch((e) => setError(String(e)));
  }, [query, from, to]);

  useEffect(() => {
    const t = setTimeout(load, 200);
    return () => clearTimeout(t);
  }, [load]);

  const open = (id: string) => api.getSale(id).then(setDetail).catch((e) => setError(String(e)));

  return (
    <div>
      <PageHeader title="المبيعات" description="الفواتير المكتملة لا تُعدَّل ولا تُحذف. التصحيح يتم بالمرتجعات." />
      <div className={`${cardClass} mb-4 flex flex-wrap items-end gap-3 p-3`}>
        <label className="text-sm">رقم الفاتورة
          <input className={`${inputClass} mt-1 w-48`} dir="ltr" value={query} onChange={(e) => setQuery(e.target.value)} placeholder="INV-000001" />
        </label>
        <label className="text-sm">من تاريخ
          <input type="date" className={`${inputClass} mt-1`} value={from} onChange={(e) => setFrom(e.target.value)} />
        </label>
        <label className="text-sm">إلى تاريخ
          <input type="date" className={`${inputClass} mt-1`} value={to} onChange={(e) => setTo(e.target.value)} />
        </label>
      </div>
      {notice && <div className="mb-4"><Alert kind="ok">{notice}</Alert></div>}
      {error && <div className="mb-4"><Alert kind="error">{error}</Alert></div>}

      <div className={`${cardClass} overflow-x-auto`}>
        <table className="w-full text-sm">
          <thead className="bg-slate-50 text-slate-500 dark:bg-slate-900/40"><tr>
            <th className="px-4 py-2 text-start">رقم الفاتورة</th><th className="px-4 py-2 text-start">التاريخ</th>
            <th className="px-4 py-2 text-start">البنود</th><th className="px-4 py-2 text-start">الإجمالي</th>
            <th className="px-4 py-2 text-start">العميل</th><th className="px-4 py-2 text-start">المرتجع</th><th className="px-4 py-2 text-start">الدفع</th><th className="w-16" />
          </tr></thead>
          <tbody>
            {rows.length === 0 && <tr><td colSpan={8} className="px-4 py-10 text-center text-slate-500">لا توجد فواتير</td></tr>}
            {rows.map((r) => (
              <tr key={r.id} className="border-t border-slate-100 dark:border-slate-700">
                <td className="px-4 py-2 font-medium" dir="ltr">{r.invoiceNo}</td>
                <td className="px-4 py-2">{new Date(r.createdAt).toLocaleString("ar-EG")}</td>
                <td className="px-4 py-2">{r.itemCount.toLocaleString("ar-EG")}</td>
                <td className="px-4 py-2 font-semibold">{formatMoney(r.totalMinor)}</td>
                <td className="px-4 py-2">{r.customerName ?? "—"}</td>
                <td className="px-4 py-2">{r.returnedMinor > 0 ? <span className="text-amber-600">{formatMoney(r.returnedMinor)}</span> : "—"}</td>
                <td className="px-4 py-2">{r.paymentStatus === "paid" ? "مدفوعة" : r.paymentStatus === "partial" ? "جزئي" : "غير مدفوعة"}</td>
                <td className="px-2"><button className={btnGhost} aria-label="عرض" onClick={() => open(r.id)}><Eye size={14} /></button></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {detail && !returning && (
        <Modal wide title={`فاتورة ${detail.invoiceNo}`} onClose={() => setDetail(null)}>
          <div className="space-y-4 text-sm">
            <div className="text-slate-500">{new Date(detail.createdAt).toLocaleString("ar-EG")}</div>
            <table className="w-full">
              <thead className="text-slate-500"><tr>
                <th className="py-1 text-start">البند</th><th className="text-start">الكمية</th><th className="text-start">السعر</th>
                <th className="text-start">الخصم</th><th className="text-start">الإجمالي</th><th className="text-start">مرتجع</th>
              </tr></thead>
              <tbody>
                {detail.items.map((i) => (
                  <tr key={i.id} className="border-t border-slate-100 dark:border-slate-700">
                    <td className="py-1">{i.nameAr}{i.variantName ? ` - ${i.variantName}` : ""}</td>
                    <td>{formatQty(i.qtyMilli)} {i.unitName}</td>
                    <td>{formatMoney(i.unitPriceMinor)}</td>
                    <td>{formatMoney(i.discountMinor + i.invoiceDiscountMinor)}</td>
                    <td>{formatMoney(i.totalMinor)}</td>
                    <td>{i.returnedQtyMilli > 0 ? formatQty(i.returnedQtyMilli) : "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-1">
                <div className="font-semibold">المدفوعات</div>
                {detail.payments.map((p, i) => <div key={i} className="flex justify-between"><span>{methodLabel[p.method] ?? p.method}</span><span>{formatMoney(p.amountMinor)}</span></div>)}
                {detail.customerName && <div className="flex justify-between"><span>العميل</span><span>{detail.customerName}</span></div>}
                {detail.dueMinor > 0 && <div className="flex justify-between font-semibold text-red-600"><span>آجل</span><span>{formatMoney(detail.dueMinor)}</span></div>}
                {detail.changeMinor > 0 && <div className="flex justify-between text-slate-500"><span>الباقي للعميل</span><span>{formatMoney(detail.changeMinor)}</span></div>}
              </div>
              <div className="space-y-1">
                <div className="flex justify-between"><span>الإجمالي قبل الخصم</span><span>{formatMoney(detail.subtotalMinor)}</span></div>
                <div className="flex justify-between"><span>الخصم</span><span>-{formatMoney(detail.itemDiscountMinor + detail.invoiceDiscountMinor)}</span></div>
                <div className="flex justify-between"><span>منها ضريبة</span><span>{formatMoney(detail.taxMinor)}</span></div>
                <div className="flex justify-between text-base font-bold"><span>الإجمالي</span><span>{formatMoney(detail.totalMinor)}</span></div>
              </div>
            </div>
            {detail.returns.length > 0 && (
              <div className="space-y-1 rounded-lg bg-amber-50 p-3 dark:bg-amber-900/20">
                <div className="font-semibold">سجل المرتجعات</div>
                {detail.returns.map((r) => (
                  <div key={r.id} className="flex justify-between">
                    <span dir="ltr">{r.returnNo}</span>
                    <span>{new Date(r.createdAt).toLocaleString("ar-EG")} · {methodLabel[r.refundMethod]} {r.reason && `· ${r.reason}`}</span>
                    <span>{formatMoney(r.refundMinor)}</span>
                  </div>
                ))}
              </div>
            )}
            <div className="flex justify-end gap-2">
              <button className={btnGhost} onClick={() => setReceiptFor(detail.id)}>إعادة طباعة الإيصال</button>
              <button className={btnPrimary} onClick={() => setReturning(true)}>مرتجع</button>
            </div>
          </div>
        </Modal>
      )}
      {detail && returning && (
        <ReturnDialog
          sale={detail}
          onClose={() => setReturning(false)}
          onDone={(refund, no) => {
            setReturning(false);
            setDetail(null);
            setNotice(`تم تسجيل المرتجع ${no} بمبلغ ${formatMoney(refund)}`);
            load();
          }}
        />
      )}
      {receiptFor && <ReceiptDialog saleId={receiptFor} onClose={() => setReceiptFor(null)} />}
    </div>
  );
}
