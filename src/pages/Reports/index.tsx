import { Download } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import Alert from "../../components/Alert";
import PageHeader from "../../components/PageHeader";
import StatCard from "../../components/StatCard";
import Tabs from "../../components/Tabs";
import { btnGhost, cardClass, inputClass } from "../../components/ui";
import { formatBp, formatMoney, formatQty } from "../../lib/format";
import {
  api,
  type CsvKind,
  type DateRange,
  type DebtsSummary,
  type InventorySummary,
  type ProductProfitRow,
  type ProfitLoss,
  type PurchasesReport,
  type SalesReport,
} from "../../services/api";

type Tab = "sales" | "products" | "purchases" | "pnl" | "debts" | "inventory";
const tabs = [
  { id: "sales", label: "المبيعات" },
  { id: "products", label: "ربح الأصناف" },
  { id: "purchases", label: "المشتريات" },
  { id: "pnl", label: "الأرباح والخسائر" },
  { id: "debts", label: "الديون" },
  { id: "inventory", label: "المخزون" },
] as const;

const ymd = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
const addDays = (d: Date, n: number) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);

type Preset = "today" | "yesterday" | "week" | "month" | "lastMonth" | "all" | "custom";
const presetLabels: [Preset, string][] = [
  ["today", "اليوم"], ["yesterday", "أمس"], ["week", "آخر 7 أيام"], ["month", "هذا الشهر"],
  ["lastMonth", "الشهر الماضي"], ["all", "الكل"], ["custom", "مخصص"],
];

function presetRange(p: Preset): DateRange {
  const now = new Date();
  switch (p) {
    case "today": return { dateFrom: ymd(now), dateTo: ymd(now) };
    case "yesterday": return { dateFrom: ymd(addDays(now, -1)), dateTo: ymd(addDays(now, -1)) };
    case "week": return { dateFrom: ymd(addDays(now, -6)), dateTo: ymd(now) };
    case "month": return { dateFrom: ymd(new Date(now.getFullYear(), now.getMonth(), 1)), dateTo: ymd(now) };
    case "lastMonth": return { dateFrom: ymd(new Date(now.getFullYear(), now.getMonth() - 1, 1)), dateTo: ymd(new Date(now.getFullYear(), now.getMonth(), 0)) };
    default: return { dateFrom: null, dateTo: null };
  }
}

/** حفظ نص CSV كملف (UTF-8 مع BOM ليفتح بالعربية في Excel). */
function downloadCsv(text: string, name: string) {
  const blob = new Blob(["﻿", text], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

const th = "px-3 py-2 text-start";
const td = "px-3 py-2";
const Table = ({ children }: { children: React.ReactNode }) => (
  <div className={`${cardClass} overflow-x-auto`}><table className="w-full text-sm">{children}</table></div>
);
const Empty = ({ cols }: { cols: number }) => <tr><td colSpan={cols} className="px-4 py-10 text-center text-slate-500">لا توجد بيانات في هذه الفترة</td></tr>;
const profitClass = (v: number) => (v < 0 ? "text-red-600" : "text-teal-700 dark:text-teal-300");

export default function ReportsPage() {
  const [tab, setTab] = useState<Tab>("sales");
  const [preset, setPreset] = useState<Preset>("today");
  const [range, setRange] = useState<DateRange>(presetRange("today"));
  const [sort, setSort] = useState("profit");
  const [error, setError] = useState<string | null>(null);
  const [sales, setSales] = useState<SalesReport | null>(null);
  const [products, setProducts] = useState<ProductProfitRow[]>([]);
  const [purchases, setPurchases] = useState<PurchasesReport | null>(null);
  const [pnl, setPnl] = useState<ProfitLoss | null>(null);
  const [debts, setDebts] = useState<DebtsSummary | null>(null);
  const [inv, setInv] = useState<InventorySummary | null>(null);

  const load = useCallback(() => {
    setError(null);
    const fail = (e: unknown) => setError(String(e));
    if (tab === "sales") api.reportSales(range).then(setSales).catch(fail);
    if (tab === "products") api.reportProductProfit(range, sort, 200).then(setProducts).catch(fail);
    if (tab === "purchases") api.reportPurchases(range).then(setPurchases).catch(fail);
    if (tab === "pnl") api.reportProfitLoss(range).then(setPnl).catch(fail);
    if (tab === "debts") api.reportDebts().then(setDebts).catch(fail);
    if (tab === "inventory") api.reportInventory().then(setInv).catch(fail);
  }, [tab, range, sort]);
  useEffect(load, [load]);

  const usesRange = tab !== "debts" && tab !== "inventory";
  const exportKind: CsvKind | null =
    tab === "sales" ? "sales_daily" : tab === "products" ? "product_profit" : tab === "purchases" ? "purchases_suppliers"
    : tab === "pnl" ? "profit_loss" : tab === "debts" ? "debts" : null;

  async function exportNow(kind: CsvKind) {
    try {
      const text = await api.reportExportCsv(kind, range);
      downloadCsv(text, `${kind}-${range.dateFrom ?? "all"}_${range.dateTo ?? "all"}.csv`);
    } catch (e) {
      setError(String(e));
    }
  }

  function pick(p: Preset) {
    setPreset(p);
    if (p !== "custom") setRange(presetRange(p));
  }

  return (
    <div>
      <PageHeader title="التقارير" />
      <Tabs tabs={tabs} value={tab} onChange={setTab} />
      {error && <Alert kind="error">{error}</Alert>}

      <div className="mb-4 flex flex-wrap items-end gap-2">
        {usesRange && (
          <>
            {presetLabels.map(([id, label]) => (
              <button key={id} className={`${btnGhost} ${preset === id ? "!border-teal-600 !text-teal-700 dark:!text-teal-300" : ""}`} onClick={() => pick(id)}>{label}</button>
            ))}
            {preset === "custom" && (
              <>
                <label className="text-xs">من
                  <input type="date" className={`${inputClass} mt-1`} value={range.dateFrom ?? ""} onChange={(e) => setRange({ ...range, dateFrom: e.target.value || null })} />
                </label>
                <label className="text-xs">إلى
                  <input type="date" className={`${inputClass} mt-1`} value={range.dateTo ?? ""} onChange={(e) => setRange({ ...range, dateTo: e.target.value || null })} />
                </label>
              </>
            )}
          </>
        )}
        {tab === "products" && (
          <select aria-label="الترتيب" className={`${inputClass} !w-auto`} value={sort} onChange={(e) => setSort(e.target.value)}>
            <option value="profit">الأعلى ربحًا</option>
            <option value="revenue">الأعلى إيرادًا</option>
            <option value="qty">الأكثر مبيعًا</option>
          </select>
        )}
        <span className="flex-1" />
        {exportKind && <button className={btnGhost} onClick={() => void exportNow(exportKind)}><Download size={16} /> تصدير CSV</button>}
        {tab === "purchases" && <button className={btnGhost} onClick={() => void exportNow("purchases_products")}><Download size={16} /> تصدير الأصناف</button>}
      </div>

      {tab === "sales" && sales && (
        <div className="space-y-4">
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <StatCard label="عدد الفواتير" value={sales.invoices.toLocaleString("ar-EG")} />
            <StatCard label="إجمالي المبيعات" value={formatMoney(sales.totalMinor)} />
            <StatCard label="المرتجعات" value={formatMoney(sales.returnsMinor)} />
            <StatCard label="صافي المبيعات" value={formatMoney(sales.netSalesMinor)} />
            <StatCard label="الخصومات" value={formatMoney(sales.discountsMinor)} />
            <StatCard label="الضريبة" value={formatMoney(sales.taxMinor)} />
            <StatCard label="تكلفة المباع" value={formatMoney(sales.cogsMinor)} />
            <StatCard label="الربح" value={`${formatMoney(sales.profitMinor)} (${formatBp(sales.marginBp)})`} />
          </div>
          <Table>
            <thead className="bg-slate-50 text-slate-500 dark:bg-slate-900/40"><tr>
              <th className={th}>اليوم</th><th className={th}>الفواتير</th><th className={th}>الإجمالي</th><th className={th}>المرتجعات</th><th className={th}>التكلفة</th><th className={th}>الربح</th>
            </tr></thead>
            <tbody>
              {sales.byDay.length === 0 && <Empty cols={6} />}
              {[...sales.byDay].reverse().map((d) => (
                <tr key={d.day} className="border-t border-slate-100 dark:border-slate-700">
                  <td className={td}>{new Date(`${d.day}T00:00:00`).toLocaleDateString("ar-EG")}</td>
                  <td className={td}>{d.invoices.toLocaleString("ar-EG")}</td>
                  <td className={td}>{formatMoney(d.totalMinor)}</td>
                  <td className={td}>{d.returnsMinor > 0 ? formatMoney(d.returnsMinor) : "—"}</td>
                  <td className={td}>{formatMoney(d.cogsMinor)}</td>
                  <td className={`${td} font-semibold ${profitClass(d.profitMinor)}`}>{formatMoney(d.profitMinor)}</td>
                </tr>
              ))}
            </tbody>
          </Table>
          {sales.byMethod.length > 0 && (
            <div className="grid gap-3 sm:grid-cols-2">
              {sales.byMethod.map((m) => (
                <StatCard key={m.method} label={m.method === "cash" ? "نقدي (مقبوض - مرتجع)" : "كارت (مقبوض - مرتجع)"} value={`${formatMoney(m.receivedMinor)} - ${formatMoney(m.refundedMinor)}`} />
              ))}
            </div>
          )}
        </div>
      )}

      {tab === "products" && (
        <Table>
          <thead className="bg-slate-50 text-slate-500 dark:bg-slate-900/40"><tr>
            <th className={th}>الصنف</th><th className={th}>الكمية</th><th className={th}>الإيراد</th><th className={th}>التكلفة</th><th className={th}>الربح</th><th className={th}>الهامش</th>
          </tr></thead>
          <tbody>
            {products.length === 0 && <Empty cols={6} />}
            {products.map((p) => (
              <tr key={`${p.productId}-${p.variantId ?? ""}`} className="border-t border-slate-100 dark:border-slate-700">
                <td className={td}>{p.nameAr}{p.variantName ? ` - ${p.variantName}` : ""}</td>
                <td className={td}>{formatQty(p.qtyBaseMilli)}</td>
                <td className={td}>{formatMoney(p.revenueMinor)}</td>
                <td className={td}>{formatMoney(p.costMinor)}</td>
                <td className={`${td} font-semibold ${profitClass(p.profitMinor)}`}>{formatMoney(p.profitMinor)}</td>
                <td className={td}>{formatBp(p.marginBp)}</td>
              </tr>
            ))}
          </tbody>
        </Table>
      )}

      {tab === "purchases" && purchases && (
        <div className="space-y-4">
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <StatCard label="فواتير مستلمة" value={purchases.invoices.toLocaleString("ar-EG")} />
            <StatCard label="إجمالي المشتريات" value={formatMoney(purchases.totalMinor)} />
            <StatCard label="المرتجعات" value={formatMoney(purchases.returnedMinor)} />
            <StatCard label="المدفوع للموردين" value={formatMoney(purchases.paidMinor)} />
          </div>
          <Table>
            <thead className="bg-slate-50 text-slate-500 dark:bg-slate-900/40"><tr>
              <th className={th}>المورد</th><th className={th}>الفواتير</th><th className={th}>الإجمالي</th><th className={th}>المرتجعات</th><th className={th}>المدفوع</th>
            </tr></thead>
            <tbody>
              {purchases.bySupplier.length === 0 && <Empty cols={5} />}
              {purchases.bySupplier.map((s) => (
                <tr key={s.supplierId} className="border-t border-slate-100 dark:border-slate-700">
                  <td className={td}>{s.nameAr}</td><td className={td}>{s.invoices.toLocaleString("ar-EG")}</td>
                  <td className={td}>{formatMoney(s.totalMinor)}</td><td className={td}>{formatMoney(s.returnedMinor)}</td><td className={td}>{formatMoney(s.paidMinor)}</td>
                </tr>
              ))}
            </tbody>
          </Table>
          <Table>
            <thead className="bg-slate-50 text-slate-500 dark:bg-slate-900/40"><tr>
              <th className={th}>الصنف المشترى</th><th className={th}>الكمية</th><th className={th}>القيمة</th>
            </tr></thead>
            <tbody>
              {purchases.byProduct.length === 0 && <Empty cols={3} />}
              {purchases.byProduct.map((p) => (
                <tr key={`${p.productId}-${p.variantId ?? ""}`} className="border-t border-slate-100 dark:border-slate-700">
                  <td className={td}>{p.nameAr}{p.variantName ? ` - ${p.variantName}` : ""}</td><td className={td}>{formatQty(p.qtyBaseMilli)}</td><td className={td}>{formatMoney(p.valueMinor)}</td>
                </tr>
              ))}
            </tbody>
          </Table>
        </div>
      )}

      {tab === "pnl" && pnl && (
        <div className="space-y-4">
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <StatCard label="صافي المبيعات (بدون ضريبة)" value={formatMoney(pnl.revenueMinor)} />
            <StatCard label="تكلفة المباع" value={formatMoney(pnl.cogsMinor)} />
            <StatCard label="مجمل الربح" value={`${formatMoney(pnl.grossProfitMinor)} (${formatBp(pnl.grossMarginBp)})`} />
            <StatCard label="المصروفات" value={formatMoney(pnl.expensesMinor)} />
          </div>
          <div className={`${cardClass} p-4`}>
            <div className="text-sm text-slate-500">صافي الربح</div>
            <div className={`text-2xl font-bold ${profitClass(pnl.netProfitMinor)}`}>{formatMoney(pnl.netProfitMinor)}</div>
          </div>
          <Table>
            <thead className="bg-slate-50 text-slate-500 dark:bg-slate-900/40"><tr><th className={th}>تصنيف المصروف</th><th className={th}>المبلغ</th></tr></thead>
            <tbody>
              {pnl.expensesByCategory.length === 0 && <Empty cols={2} />}
              {pnl.expensesByCategory.map((e) => (
                <tr key={e.category} className="border-t border-slate-100 dark:border-slate-700"><td className={td}>{e.category}</td><td className={td}>{formatMoney(e.amountMinor)}</td></tr>
              ))}
            </tbody>
          </Table>
        </div>
      )}

      {tab === "debts" && debts && (
        <div className="space-y-4">
          <div className="grid gap-3 sm:grid-cols-2">
            <StatCard label="مستحق لنا على العملاء" value={formatMoney(debts.receivablesMinor)} />
            <StatCard label="مستحق علينا للموردين" value={formatMoney(debts.payablesMinor)} />
          </div>
          {([["العملاء المدينون", debts.customers], ["الموردون الدائنون", debts.suppliers]] as const).map(([title, rows]) => (
            <Table key={title}>
              <thead className="bg-slate-50 text-slate-500 dark:bg-slate-900/40"><tr><th className={th}>{title}</th><th className={th}>الهاتف</th><th className={th}>المبلغ</th></tr></thead>
              <tbody>
                {rows.length === 0 && <Empty cols={3} />}
                {rows.map((r) => (
                  <tr key={r.id} className="border-t border-slate-100 dark:border-slate-700"><td className={td}>{r.nameAr}</td><td className={td} dir="ltr">{r.phone || "—"}</td><td className={`${td} font-semibold`}>{formatMoney(r.amountMinor)}</td></tr>
                ))}
              </tbody>
            </Table>
          ))}
        </div>
      )}

      {tab === "inventory" && inv && (
        <div className="space-y-4">
          <div className="grid gap-3 sm:grid-cols-3">
            <StatCard label="قيمة المخزون (بالتكلفة)" value={formatMoney(inv.totalValueMinor)} />
            <StatCard label="أصناف بها رصيد" value={inv.itemsInStock.toLocaleString("ar-EG")} />
            <StatCard label="أصناف تحت حد الطلب" value={inv.lowStockCount.toLocaleString("ar-EG")} />
          </div>
          <Table>
            <thead className="bg-slate-50 text-slate-500 dark:bg-slate-900/40"><tr><th className={th}>المخزن</th><th className={th}>القيمة</th></tr></thead>
            <tbody>
              {inv.warehouses.length === 0 && <Empty cols={2} />}
              {inv.warehouses.map((w) => (
                <tr key={w.nameAr} className="border-t border-slate-100 dark:border-slate-700"><td className={td}>{w.nameAr}</td><td className={td}>{formatMoney(w.valueMinor)}</td></tr>
              ))}
            </tbody>
          </Table>
        </div>
      )}
    </div>
  );
}
