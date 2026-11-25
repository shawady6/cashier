import { useCallback, useEffect, useState } from "react";
import Alert from "../../components/Alert";
import { btnGhost, btnPrimary, cardClass, inputClass } from "../../components/ui";
import { formatMoney } from "../../lib/format";
import { api, type SessionSummary } from "../../services/api";

function Row({ label, value, strong }: { label: string; value: number; strong?: boolean }) {
  return <div className={`flex justify-between ${strong ? "font-bold" : ""}`}><span>{label}</span><span>{formatMoney(value)}</span></div>;
}

function Breakdown({ s }: { s: SessionSummary }) {
  return (
    <div className="grid gap-1 text-sm sm:w-96">
      <Row label="العهدة الافتتاحية" value={s.openingMinor} />
      <Row label="مبيعات نقدية" value={s.cashSalesMinor} />
      <Row label="سداد عملاء نقدًا" value={s.customerPaymentsMinor} />
      <Row label="إدخال يدوي" value={s.cashInMinor} />
      <Row label="مرتجعات نقدية" value={-s.cashReturnsMinor} />
      <Row label="مصروفات نقدية" value={-s.cashExpensesMinor} />
      <Row label="دفعات موردين نقدًا" value={-s.supplierPaymentsMinor} />
      <Row label="إخراج يدوي" value={-s.cashOutMinor} />
      <hr className="my-1 border-slate-200 dark:border-slate-700" />
      <Row label="المتوقع في الدرج" value={s.expectedCashMinor} strong />
      <Row label="حركة الكارت" value={s.cardTotalMinor} />
      {s.countedCashMinor !== null && <Row label="المعدود فعليًا" value={s.countedCashMinor} />}
      {s.differenceMinor !== null && (
        <div className={`flex justify-between font-bold ${s.differenceMinor === 0 ? "" : "text-red-600"}`}>
          <span>الفرق</span><span>{formatMoney(s.differenceMinor)}</span>
        </div>
      )}
    </div>
  );
}

export default function SessionTab() {
  const [cur, setCur] = useState<SessionSummary | null>(null);
  const [history, setHistory] = useState<SessionSummary[]>([]);
  const [opening, setOpening] = useState("0");
  const [counted, setCounted] = useState("");
  const [entry, setEntry] = useState({ direction: "in" as "in" | "out", amount: "", reason: "" });
  const [error, setError] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);

  const load = useCallback(() => {
    api.currentCashSession().then(setCur).catch((e) => setError(String(e)));
    api.listCashSessions().then((l) => setHistory(l.filter((x) => x.status === "closed"))).catch(() => undefined);
  }, []);
  useEffect(load, [load]);

  async function run(fn: () => Promise<unknown>, ok: string) {
    try {
      await fn();
      setError(null);
      setMsg(ok);
      load();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="space-y-4">
      {error && <Alert kind="error">{error}</Alert>}
      {msg && <Alert kind="ok">{msg}</Alert>}
      {!cur ? (
        <form className={`${cardClass} max-w-md space-y-3 p-4`} onSubmit={(e) => { e.preventDefault(); void run(() => api.openCashSession(opening), "تم فتح الوردية"); }}>
          <div className="font-semibold">لا توجد وردية مفتوحة</div>
          <label className="block text-sm">العهدة الافتتاحية في الدرج (ج.م)
            <input className={`${inputClass} mt-1`} dir="ltr" inputMode="decimal" value={opening} onChange={(e) => setOpening(e.target.value)} />
          </label>
          <button className={btnPrimary}>فتح وردية</button>
        </form>
      ) : (
        <div className="grid gap-4 md:grid-cols-2">
          <div className={`${cardClass} space-y-3 p-4`}>
            <div className="font-semibold">الوردية الحالية <span className="text-xs text-slate-400" dir="ltr">{cur.openedAt.slice(0, 16).replace("T", " ")}</span></div>
            <Breakdown s={cur} />
          </div>
          <div className="space-y-4">
            <form className={`${cardClass} space-y-3 p-4`} onSubmit={(e) => { e.preventDefault(); void run(async () => { await api.addCashEntry(entry); setEntry({ ...entry, amount: "", reason: "" }); }, "تم تسجيل الحركة"); }}>
              <div className="font-semibold">إدخال / إخراج نقدية</div>
              <div className="flex gap-2">
                <select className={`${inputClass} w-28`} value={entry.direction} onChange={(e) => setEntry({ ...entry, direction: e.target.value as "in" | "out" })}><option value="in">إدخال</option><option value="out">إخراج</option></select>
                <input className={inputClass} dir="ltr" inputMode="decimal" placeholder="المبلغ" value={entry.amount} onChange={(e) => setEntry({ ...entry, amount: e.target.value })} />
              </div>
              <input className={inputClass} placeholder="السبب (مطلوب)" value={entry.reason} onChange={(e) => setEntry({ ...entry, reason: e.target.value })} />
              <button className={btnGhost} disabled={!entry.amount.trim() || !entry.reason.trim()}>تسجيل</button>
            </form>
            <form className={`${cardClass} space-y-3 p-4`} onSubmit={(e) => { e.preventDefault(); if (window.confirm("إغلاق الوردية؟ لا يمكن تعديلها بعد الإغلاق.")) void run(() => api.closeCashSession(cur.id, counted), "تم إغلاق الوردية"); }}>
              <div className="font-semibold">إغلاق الوردية</div>
              <label className="block text-sm">النقدية المعدودة فعليًا في الدرج (ج.م)
                <input className={`${inputClass} mt-1`} dir="ltr" inputMode="decimal" value={counted} onChange={(e) => setCounted(e.target.value)} />
              </label>
              <button className={btnPrimary} disabled={!counted.trim()}>إغلاق وحساب الفرق</button>
            </form>
          </div>
        </div>
      )}
      {history.length > 0 && (
        <div className={`${cardClass} overflow-x-auto`}>
          <table className="w-full text-sm">
            <thead className="text-slate-500"><tr><th className="p-3 text-start">الفتح</th><th className="p-3 text-start">الإغلاق</th><th className="p-3 text-start">المتوقع</th><th className="p-3 text-start">المعدود</th><th className="p-3 text-start">الفرق</th></tr></thead>
            <tbody>
              {history.map((s) => (
                <tr key={s.id} className="border-t border-slate-100 dark:border-slate-700">
                  <td className="p-3" dir="ltr">{s.openedAt.slice(0, 16).replace("T", " ")}</td>
                  <td className="p-3" dir="ltr">{s.closedAt?.slice(0, 16).replace("T", " ")}</td>
                  <td className="p-3">{formatMoney(s.expectedCashMinor)}</td>
                  <td className="p-3">{formatMoney(s.countedCashMinor ?? 0)}</td>
                  <td className={`p-3 font-semibold ${s.differenceMinor ? "text-red-600" : ""}`}>{formatMoney(s.differenceMinor ?? 0)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
