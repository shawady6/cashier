import { useEffect, useState } from "react";
import Alert from "../../components/Alert";
import { cardClass, inputClass } from "../../components/ui";
import { formatMoney, formatQty } from "../../lib/format";
import { api, type BalancesReport, type LowStockRow, type Warehouse } from "../../services/api";

export default function BalancesTab() {
  const [wh, setWh] = useState<Warehouse[]>([]);
  const [warehouseId, setWarehouseId] = useState("");
  const [query, setQuery] = useState("");
  const [report, setReport] = useState<BalancesReport | null>(null);
  const [low, setLow] = useState<LowStockRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.listWarehouses().then(setWh).catch((e) => setError(String(e)));
    api.lowStockReport().then(setLow).catch(() => undefined);
  }, []);
  useEffect(() => {
    const t = setTimeout(() => {
      api
        .stockBalances({ warehouseId: warehouseId || undefined, query: query || undefined })
        .then(setReport)
        .catch((e) => setError(String(e)));
    }, 200);
    return () => clearTimeout(t);
  }, [warehouseId, query]);

  return (
    <div className="space-y-4">
      {error && <Alert kind="error">{error}</Alert>}
      {low.length > 0 && (
        <Alert kind="error">
          أصناف وصلت للحد الأدنى ({low.length}): {low.slice(0, 6).map((l) => `${l.nameAr} (${formatQty(l.stockQtyMilli)}/${formatQty(l.minStockQtyMilli)})`).join("، ")}
          {low.length > 6 ? " ..." : ""}
        </Alert>
      )}
      <div className="flex flex-wrap gap-2">
        <select className={`${inputClass} max-w-xs`} value={warehouseId} onChange={(e) => setWarehouseId(e.target.value)}>
          <option value="">كل المخازن</option>
          {wh.map((w) => <option key={w.id} value={w.id}>{w.nameAr}</option>)}
        </select>
        <input className={`${inputClass} max-w-xs`} placeholder="بحث..." value={query} onChange={(e) => setQuery(e.target.value)} />
        <div className="ms-auto text-sm text-slate-500">قيمة المخزون: <b>{formatMoney(report?.totalValueMinor ?? 0)}</b></div>
      </div>
      <div className={`${cardClass} overflow-x-auto`}>
        <table className="w-full text-sm">
          <thead className="text-slate-500">
            <tr>
              <th className="p-3 text-start">الصنف</th>
              <th className="p-3 text-start">المخزن</th>
              <th className="p-3 text-start">الكمية</th>
              <th className="p-3 text-start">متوسط التكلفة</th>
              <th className="p-3 text-start">القيمة</th>
            </tr>
          </thead>
          <tbody>
            {(report?.rows ?? []).length === 0 && (
              <tr><td colSpan={5} className="p-6 text-center text-slate-400">لا توجد أرصدة.</td></tr>
            )}
            {(report?.rows ?? []).map((r) => (
              <tr key={`${r.warehouseId}-${r.productId}-${r.variantId ?? ""}`} className="border-t border-slate-100 dark:border-slate-700">
                <td className="p-3">{r.nameAr}{r.variantName ? ` — ${r.variantName}` : ""}</td>
                <td className="p-3">{r.warehouseName}</td>
                <td className="p-3">{formatQty(r.qtyMilli)} {r.baseUnitName}</td>
                <td className="p-3">{formatMoney(r.avgCostMinor)}</td>
                <td className="p-3">{formatMoney(r.valueMinor)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
