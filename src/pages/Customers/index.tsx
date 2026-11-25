import { Plus } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import Alert from "../../components/Alert";
import Modal from "../../components/Modal";
import PageHeader from "../../components/PageHeader";
import { btnGhost, btnPrimary, cardClass, inputClass } from "../../components/ui";
import { formatMoney, minorToInput } from "../../lib/format";
import { api, type Customer, type CustomerInput, type Statement } from "../../services/api";

const empty: CustomerInput = { nameAr: "", phone: "", address: "", notes: "", creditLimit: "", isActive: true };
const kindLabel = { sale: "فاتورة آجل", payment: "سداد", return: "مرتجع" } as const;

export default function CustomersPage() {
  const [rows, setRows] = useState<Customer[]>([]);
  const [edit, setEdit] = useState<CustomerInput | null>(null);
  const [pay, setPay] = useState<Customer | null>(null);
  const [payAmount, setPayAmount] = useState("");
  const [payMethod, setPayMethod] = useState<"cash" | "card">("cash");
  const [stmt, setStmt] = useState<Statement | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);

  const load = useCallback(() => { api.listCustomers().then(setRows).catch((e) => setError(String(e))); }, []);
  useEffect(load, [load]);

  async function run(fn: () => Promise<unknown>, after?: () => void, ok?: string) {
    try {
      await fn();
      setError(null);
      if (ok) setMsg(ok);
      after?.();
      load();
    } catch (e) {
      setError(String(e));
    }
  }

  const totalDue = rows.reduce((s, c) => s + c.balanceMinor, 0);

  return (
    <div className="max-w-5xl">
      <PageHeader title="العملاء" description="حسابات العملاء والبيع الآجل. الرصيد يُحسب من الفواتير الآجلة والسداد والمرتجعات." />
      {error && !edit && !pay && <Alert kind="error">{error}</Alert>}
      {msg && <Alert kind="ok">{msg}</Alert>}
      <div className="mb-3 flex flex-wrap items-center gap-3">
        <button className={btnPrimary} onClick={() => { setError(null); setEdit(empty); }}><Plus size={16} /> عميل جديد</button>
        <span className="ms-auto text-sm text-slate-500">إجمالي المستحق على العملاء: <b>{formatMoney(totalDue)}</b></span>
      </div>
      <div className={`${cardClass} overflow-x-auto`}>
        <table className="w-full text-sm">
          <thead className="text-slate-500">
            <tr><th className="p-3 text-start">الاسم</th><th className="p-3 text-start">الهاتف</th><th className="p-3 text-start">الحد الائتماني</th><th className="p-3 text-start">المستحق</th><th /></tr>
          </thead>
          <tbody>
            {rows.length === 0 && <tr><td colSpan={5} className="p-6 text-center text-slate-400">لا يوجد عملاء بعد.</td></tr>}
            {rows.map((c) => (
              <tr key={c.id} className="border-t border-slate-100 dark:border-slate-700">
                <td className="p-3">{c.nameAr}{c.isActive ? "" : " (موقوف)"}</td>
                <td className="p-3" dir="ltr">{c.phone}</td>
                <td className="p-3">{c.creditLimitMinor > 0 ? formatMoney(c.creditLimitMinor) : "بلا حد"}</td>
                <td className={`p-3 font-semibold ${c.balanceMinor > 0 ? "text-red-600" : ""}`}>{formatMoney(c.balanceMinor)}</td>
                <td className="p-3 text-end whitespace-nowrap">
                  <button className={btnGhost} onClick={() => api.customerStatement(c.id).then(setStmt).catch((e) => setError(String(e)))}>كشف حساب</button>
                  {c.balanceMinor > 0 && <button className={btnGhost} onClick={() => { setError(null); setMsg(null); setPayAmount(minorToInput(c.balanceMinor)); setPay(c); }}>سداد</button>}
                  <button className={btnGhost} onClick={() => { setError(null); setEdit({ id: c.id, nameAr: c.nameAr, phone: c.phone, address: c.address, notes: c.notes, creditLimit: c.creditLimitMinor ? minorToInput(c.creditLimitMinor) : "", isActive: c.isActive }); }}>تعديل</button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {edit && (
        <Modal title={edit.id ? "تعديل عميل" : "عميل جديد"} onClose={() => setEdit(null)}>
          <form className="space-y-3" onSubmit={(e) => { e.preventDefault(); void run(() => api.saveCustomer(edit), () => setEdit(null)); }}>
            {error && <Alert kind="error">{error}</Alert>}
            <label className="block text-sm">الاسم<input className={`${inputClass} mt-1`} value={edit.nameAr} onChange={(e) => setEdit({ ...edit, nameAr: e.target.value })} autoFocus /></label>
            <label className="block text-sm">الهاتف<input className={`${inputClass} mt-1`} dir="ltr" value={edit.phone} onChange={(e) => setEdit({ ...edit, phone: e.target.value })} /></label>
            <label className="block text-sm">العنوان<input className={`${inputClass} mt-1`} value={edit.address} onChange={(e) => setEdit({ ...edit, address: e.target.value })} /></label>
            <label className="block text-sm">الحد الائتماني (ج.م، فارغ = بلا حد)<input className={`${inputClass} mt-1`} dir="ltr" inputMode="decimal" value={edit.creditLimit} onChange={(e) => setEdit({ ...edit, creditLimit: e.target.value })} /></label>
            <label className="block text-sm">ملاحظات<textarea className={`${inputClass} mt-1`} value={edit.notes} onChange={(e) => setEdit({ ...edit, notes: e.target.value })} /></label>
            <label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={edit.isActive} onChange={(e) => setEdit({ ...edit, isActive: e.target.checked })} /> نشط</label>
            <div className="flex justify-end gap-2"><button type="button" className={btnGhost} onClick={() => setEdit(null)}>إلغاء</button><button className={btnPrimary}>حفظ</button></div>
          </form>
        </Modal>
      )}

      {pay && (
        <Modal title={`سداد من ${pay.nameAr}`} onClose={() => setPay(null)}>
          <form className="space-y-3" onSubmit={(e) => { e.preventDefault(); void run(() => api.addCustomerPayment({ customerId: pay.id, method: payMethod, amount: payAmount }), () => setPay(null), "تم تسجيل السداد"); }}>
            {error && <Alert kind="error">{error}</Alert>}
            <p className="text-sm text-slate-500">المستحق: {formatMoney(pay.balanceMinor)}</p>
            <div className="flex gap-2">
              <input className={inputClass} dir="ltr" inputMode="decimal" value={payAmount} onChange={(e) => setPayAmount(e.target.value)} autoFocus />
              <select className={`${inputClass} w-28`} value={payMethod} onChange={(e) => setPayMethod(e.target.value as "cash" | "card")}><option value="cash">نقدي</option><option value="card">كارت</option></select>
            </div>
            <div className="flex justify-end gap-2"><button type="button" className={btnGhost} onClick={() => setPay(null)}>إلغاء</button><button className={btnPrimary} disabled={!payAmount.trim()}>تسجيل</button></div>
          </form>
        </Modal>
      )}

      {stmt && (
        <Modal title={`كشف حساب ${stmt.customer.nameAr}`} onClose={() => setStmt(null)} wide>
          <table className="w-full text-sm">
            <thead className="text-slate-500"><tr><th className="p-2 text-start">التاريخ</th><th className="p-2 text-start">البيان</th><th className="p-2 text-start">مدين</th><th className="p-2 text-start">دائن</th><th className="p-2 text-start">الرصيد</th></tr></thead>
            <tbody>
              {stmt.rows.length === 0 && <tr><td colSpan={5} className="p-4 text-center text-slate-400">لا توجد حركات.</td></tr>}
              {stmt.rows.map((r, i) => (
                <tr key={i} className="border-t border-slate-100 dark:border-slate-700">
                  <td className="p-2" dir="ltr">{r.date.slice(0, 16).replace("T", " ")}</td>
                  <td className="p-2">{kindLabel[r.kind]} <span dir="ltr" className="text-slate-400">{r.refNo}</span></td>
                  <td className="p-2">{r.debitMinor ? formatMoney(r.debitMinor) : ""}</td>
                  <td className="p-2">{r.creditMinor ? formatMoney(r.creditMinor) : ""}</td>
                  <td className="p-2 font-semibold">{formatMoney(r.balanceMinor)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </Modal>
      )}
    </div>
  );
}
