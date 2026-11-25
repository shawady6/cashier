import { Plus } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import Alert from "../../components/Alert";
import Modal from "../../components/Modal";
import { btnGhost, btnPrimary, cardClass, inputClass } from "../../components/ui";
import { formatMoney } from "../../lib/format";
import { api, type ExpenseCategory, type ExpenseReport } from "../../services/api";

export default function ExpensesTab() {
  const [cats, setCats] = useState<ExpenseCategory[]>([]);
  const [report, setReport] = useState<ExpenseReport | null>(null);
  const [filter, setFilter] = useState({ categoryId: "", dateFrom: "", dateTo: "" });
  const [form, setForm] = useState<{ categoryId: string; amount: string; method: "cash" | "card"; description: string } | null>(null);
  const [voiding, setVoiding] = useState<string | null>(null);
  const [reason, setReason] = useState("");
  const [newCat, setNewCat] = useState("");
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    api.listExpenseCategories().then(setCats).catch((e) => setError(String(e)));
    api
      .listExpenses({ categoryId: filter.categoryId || undefined, dateFrom: filter.dateFrom || undefined, dateTo: filter.dateTo || undefined })
      .then(setReport)
      .catch((e) => setError(String(e)));
  }, [filter]);
  useEffect(load, [load]);

  async function run(fn: () => Promise<unknown>, after?: () => void) {
    try {
      await fn();
      setError(null);
      after?.();
      load();
    } catch (e) {
      setError(String(e));
    }
  }
  const active = cats.filter((c) => c.isActive);

  return (
    <div className="space-y-3">
      {error && !form && !voiding && <Alert kind="error">{error}</Alert>}
      <div className="flex flex-wrap items-end gap-2">
        <button className={btnPrimary} onClick={() => { setError(null); setForm({ categoryId: active[0]?.id ?? "", amount: "", method: "cash", description: "" }); }}><Plus size={16} /> مصروف جديد</button>
        <select className={`${inputClass} max-w-[11rem]`} value={filter.categoryId} onChange={(e) => setFilter({ ...filter, categoryId: e.target.value })}>
          <option value="">كل التصنيفات</option>
          {cats.map((c) => <option key={c.id} value={c.id}>{c.nameAr}</option>)}
        </select>
        <input type="date" className={`${inputClass} max-w-[10rem]`} value={filter.dateFrom} onChange={(e) => setFilter({ ...filter, dateFrom: e.target.value })} aria-label="من تاريخ" />
        <input type="date" className={`${inputClass} max-w-[10rem]`} value={filter.dateTo} onChange={(e) => setFilter({ ...filter, dateTo: e.target.value })} aria-label="إلى تاريخ" />
        <span className="ms-auto text-sm text-slate-500">الإجمالي: <b>{formatMoney(report?.totalMinor ?? 0)}</b></span>
      </div>
      <div className={`${cardClass} overflow-x-auto`}>
        <table className="w-full text-sm">
          <thead className="text-slate-500"><tr><th className="p-3 text-start">التاريخ</th><th className="p-3 text-start">التصنيف</th><th className="p-3 text-start">البيان</th><th className="p-3 text-start">الدفع</th><th className="p-3 text-start">المبلغ</th><th /></tr></thead>
          <tbody>
            {(report?.rows ?? []).length === 0 && <tr><td colSpan={6} className="p-6 text-center text-slate-400">لا توجد مصروفات.</td></tr>}
            {(report?.rows ?? []).map((r) => (
              <tr key={r.id} className={`border-t border-slate-100 dark:border-slate-700 ${r.voidedAt ? "text-slate-400 line-through" : ""}`}>
                <td className="p-3" dir="ltr">{r.createdAt.slice(0, 16).replace("T", " ")}</td>
                <td className="p-3">{r.categoryName}</td>
                <td className="p-3">{r.description}{r.voidReason ? ` (ملغي: ${r.voidReason})` : ""}</td>
                <td className="p-3">{r.method === "cash" ? "نقدي" : "كارت"}</td>
                <td className="p-3">{formatMoney(r.amountMinor)}</td>
                <td className="p-3 text-end">{!r.voidedAt && <button className={btnGhost} onClick={() => { setError(null); setReason(""); setVoiding(r.id); }}>إلغاء</button>}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <form className="flex gap-2" onSubmit={(e) => { e.preventDefault(); void run(() => api.saveExpenseCategory({ nameAr: newCat, isActive: true }), () => setNewCat("")); }}>
        <input className={`${inputClass} max-w-xs`} placeholder="تصنيف مصروفات جديد" value={newCat} onChange={(e) => setNewCat(e.target.value)} />
        <button className={btnGhost} disabled={!newCat.trim()}>إضافة تصنيف</button>
      </form>

      {form && (
        <Modal title="مصروف جديد" onClose={() => setForm(null)}>
          <form className="space-y-3" onSubmit={(e) => { e.preventDefault(); void run(() => api.createExpense(form), () => setForm(null)); }}>
            {error && <Alert kind="error">{error}</Alert>}
            <label className="block text-sm">التصنيف
              <select className={`${inputClass} mt-1`} value={form.categoryId} onChange={(e) => setForm({ ...form, categoryId: e.target.value })}>{active.map((c) => <option key={c.id} value={c.id}>{c.nameAr}</option>)}</select>
            </label>
            <div className="flex gap-2">
              <input className={inputClass} dir="ltr" inputMode="decimal" placeholder="المبلغ (ج.م)" value={form.amount} onChange={(e) => setForm({ ...form, amount: e.target.value })} autoFocus />
              <select className={`${inputClass} w-28`} value={form.method} onChange={(e) => setForm({ ...form, method: e.target.value as "cash" | "card" })}><option value="cash">نقدي</option><option value="card">كارت</option></select>
            </div>
            <input className={inputClass} placeholder="البيان" value={form.description} onChange={(e) => setForm({ ...form, description: e.target.value })} />
            <div className="flex justify-end gap-2"><button type="button" className={btnGhost} onClick={() => setForm(null)}>إلغاء</button><button className={btnPrimary} disabled={!form.amount.trim()}>حفظ</button></div>
          </form>
        </Modal>
      )}
      {voiding && (
        <Modal title="إلغاء مصروف" onClose={() => setVoiding(null)}>
          <form className="space-y-3" onSubmit={(e) => { e.preventDefault(); void run(() => api.voidExpense(voiding, reason), () => setVoiding(null)); }}>
            {error && <Alert kind="error">{error}</Alert>}
            <p className="text-sm text-slate-500">يُسجَّل قيد عكسي في الخزينة ويبقى المصروف ظاهرًا كملغي.</p>
            <input className={inputClass} placeholder="سبب الإلغاء (مطلوب)" value={reason} onChange={(e) => setReason(e.target.value)} autoFocus />
            <div className="flex justify-end gap-2"><button type="button" className={btnGhost} onClick={() => setVoiding(null)}>رجوع</button><button className={btnPrimary} disabled={!reason.trim()}>تأكيد الإلغاء</button></div>
          </form>
        </Modal>
      )}
    </div>
  );
}
