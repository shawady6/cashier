import { AlertTriangle, Plus, ShoppingCart, Truck, Wallet } from "lucide-react";
import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import PageHeader from "../../components/PageHeader";
import StatCard from "../../components/StatCard";
import { btnGhost, btnPrimary, cardClass } from "../../components/ui";
import { formatBp, formatMoney, formatQty } from "../../lib/format";
import {
  api,
  type DateRange,
  type DebtsSummary,
  type InventorySummary,
  type LowStockRow,
  type ProductProfitRow,
  type ProfitLoss,
  type SalesReport,
  type SessionSummary,
} from "../../services/api";

const ymd = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
const addDays = (d: Date, n: number) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);
const day = (d: Date): DateRange => ({ dateFrom: ymd(d), dateTo: ymd(d) });

interface Data {
  today: SalesReport | null;
  yesterday: SalesReport | null;
  week: SalesReport | null;
  top: ProductProfitRow[];
  pnl: ProfitLoss | null;
  debts: DebtsSummary | null;
  inventory: InventorySummary | null;
  low: LowStockRow[];
  drafts: number;
  session: SessionSummary | null;
}

const empty: Data = { today: null, yesterday: null, week: null, top: [], pnl: null, debts: null, inventory: null, low: [], drafts: 0, session: null };

/** كل قسم يُحمَّل مستقلًا: فشل قسم لا يُسقط الصفحة. الأرقام جاهزة من Rust، والواجهة تعرضها فقط. */
export default function Dashboard() {
  const [d, setD] = useState<Data>(empty);
  const [now] = useState(() => new Date());

  useEffect(() => {
    const today = day(now);
    const put = <K extends keyof Data>(key: K) => (v: Data[K]) => setD((s) => ({ ...s, [key]: v }));
    const ignore = () => undefined;
    api.reportSales(today).then(put("today")).catch(ignore);
    api.reportSales(day(addDays(now, -1))).then(put("yesterday")).catch(ignore);
    api.reportSales({ dateFrom: ymd(addDays(now, -6)), dateTo: ymd(now) }).then(put("week")).catch(ignore);
    api.reportProductProfit(today, "qty", 5).then(put("top")).catch(ignore);
    api.reportProfitLoss(today).then(put("pnl")).catch(ignore);
    api.reportDebts().then(put("debts")).catch(ignore);
    api.reportInventory().then(put("inventory")).catch(ignore);
    api.lowStockReport().then(put("low")).catch(ignore);
    api.listPurchases({ status: "draft" }).then((r) => put("drafts")(r.length)).catch(ignore);
    api.currentCashSession().then(put("session")).catch(ignore);
  }, [now]);

  const week = Array.from({ length: 7 }, (_, i) => {
    const date = addDays(now, i - 6);
    const row = d.week?.byDay.find((r) => r.day === ymd(date));
    return { date, sales: row?.totalMinor ?? 0, profit: row?.profitMinor ?? 0 };
  });
  const maxBar = Math.max(1, ...week.map((w) => Math.max(w.sales, w.profit)));

  const staleSession = d.session && ymd(new Date(d.session.openedAt)) !== ymd(now);
  const alerts: { to: string; text: string; tone: "warn" | "info" }[] = [];
  if (d.low.length > 0) alerts.push({ to: "/inventory", text: `${d.low.length.toLocaleString("ar-EG")} صنف وصل لحد إعادة الطلب أو أقل`, tone: "warn" });
  if (d.drafts > 0) alerts.push({ to: "/purchases", text: `${d.drafts.toLocaleString("ar-EG")} فاتورة شراء مسودة لم تُستلم`, tone: "warn" });
  if (staleSession) alerts.push({ to: "/expenses", text: "وردية الخزينة مفتوحة من يوم سابق", tone: "warn" });
  if (d.debts && d.debts.receivablesMinor > 0) alerts.push({ to: "/customers", text: `مستحق على العملاء ${formatMoney(d.debts.receivablesMinor)}`, tone: "info" });
  if (d.debts && d.debts.payablesMinor > 0) alerts.push({ to: "/suppliers", text: `مستحق للموردين ${formatMoney(d.debts.payablesMinor)}`, tone: "info" });

  return (
    <div className="space-y-6">
      <PageHeader title="الرئيسية" />

      <div className="flex flex-wrap gap-2">
        <Link to="/pos" className={btnPrimary}><ShoppingCart size={16} /> فاتورة بيع جديدة</Link>
        <Link to="/purchases" className={btnGhost}><Truck size={16} /> فاتورة شراء</Link>
        <Link to="/products" className={btnGhost}><Plus size={16} /> إضافة منتج</Link>
        <Link to="/expenses" className={btnGhost}><Wallet size={16} /> تسجيل مصروف</Link>
      </div>

      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <StatCard
          label="مبيعات اليوم (بعد المرتجعات)"
          value={d.today ? formatMoney(d.today.netSalesMinor) : "…"}
          hint={d.yesterday ? `أمس: ${formatMoney(d.yesterday.netSalesMinor)}` : undefined}
        />
        <StatCard label="عدد الفواتير" value={d.today ? d.today.invoices.toLocaleString("ar-EG") : "…"} hint={d.yesterday ? `أمس: ${d.yesterday.invoices.toLocaleString("ar-EG")}` : undefined} />
        <StatCard
          label="ربح اليوم"
          value={d.today ? `${formatMoney(d.today.profitMinor)} (${formatBp(d.today.marginBp)})` : "…"}
          hint={d.yesterday ? `أمس: ${formatMoney(d.yesterday.profitMinor)}` : undefined}
        />
        <StatCard label="مصروفات اليوم" value={d.pnl ? formatMoney(d.pnl.expensesMinor) : "…"} />
        <StatCard label="النقدية المتوقعة في الدرج" value={d.session ? formatMoney(d.session.expectedCashMinor) : "لا توجد وردية مفتوحة"} />
        <StatCard label="قيمة المخزون" value={d.inventory ? formatMoney(d.inventory.totalValueMinor) : "…"} />
      </div>

      <div className="grid gap-4 lg:grid-cols-3">
        <section className={`${cardClass} p-4 lg:col-span-2`}>
          <h3 className="mb-3 font-semibold">آخر 7 أيام</h3>
          <div className="flex h-44 items-end gap-3" role="img" aria-label="مبيعات وأرباح آخر 7 أيام">
            {week.map((w) => (
              <div key={ymd(w.date)} className="flex flex-1 flex-col items-center gap-1">
                <div className="flex h-36 w-full items-end justify-center gap-1">
                  <div className="w-1/2 rounded-t bg-teal-500" style={{ height: `${(w.sales / maxBar) * 100}%` }} title={`المبيعات: ${formatMoney(w.sales)}`} />
                  <div className="w-1/2 rounded-t bg-amber-400" style={{ height: `${(Math.max(w.profit, 0) / maxBar) * 100}%` }} title={`الربح: ${formatMoney(w.profit)}`} />
                </div>
                <span className="text-xs text-slate-500">{w.date.toLocaleDateString("ar-EG", { weekday: "short" })}</span>
              </div>
            ))}
          </div>
          <div className="mt-2 flex gap-4 text-xs text-slate-500">
            <span className="flex items-center gap-1"><i className="inline-block h-2 w-2 rounded-sm bg-teal-500" /> المبيعات</span>
            <span className="flex items-center gap-1"><i className="inline-block h-2 w-2 rounded-sm bg-amber-400" /> الربح</span>
          </div>
        </section>

        <section className={`${cardClass} p-4`}>
          <h3 className="mb-3 font-semibold">الأكثر مبيعًا اليوم</h3>
          {d.top.length === 0 ? (
            <p className="text-sm text-slate-500">لا مبيعات بعد اليوم</p>
          ) : (
            <ol className="space-y-2 text-sm">
              {d.top.map((p, i) => (
                <li key={`${p.productId}-${p.variantId ?? ""}`} className="flex items-center justify-between gap-2">
                  <span>{(i + 1).toLocaleString("ar-EG")}. {p.nameAr}{p.variantName ? ` - ${p.variantName}` : ""}</span>
                  <span className="text-slate-500">{formatQty(p.qtyBaseMilli)}</span>
                </li>
              ))}
            </ol>
          )}
        </section>
      </div>

      {alerts.length > 0 && (
        <section className={`${cardClass} p-4`}>
          <h3 className="mb-3 font-semibold">تحتاج انتباهك</h3>
          <ul className="space-y-2 text-sm">
            {alerts.map((a) => (
              <li key={a.to + a.text}>
                <Link to={a.to} className="flex items-center gap-2 rounded-lg p-2 hover:bg-slate-50 dark:hover:bg-slate-700/40">
                  <AlertTriangle size={16} className={a.tone === "warn" ? "text-amber-500" : "text-slate-400"} />
                  {a.text}
                </Link>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}
