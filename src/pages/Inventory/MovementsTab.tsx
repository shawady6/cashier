import { useEffect, useState } from "react";
import Alert from "../../components/Alert";
import { cardClass, inputClass } from "../../components/ui";
import { formatMoney, formatQty } from "../../lib/format";
import { api, type MovementRow } from "../../services/api";

export const movementLabels: Record<string, string> = {
  adjustment: "تسوية",
  sale: "بيع",
  sale_return: "مرتجع بيع",
  purchase: "شراء",
  purchase_return: "مرتجع شراء",
  transfer_out: "تحويل صادر",
  transfer_in: "تحويل وارد",
  count: "جرد",
};

export default function MovementsTab() {
  const [type, setType] = useState("");
  const [rows, setRows] = useState<MovementRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.stockMovements({ movementType: type || undefined, limit: 300 }).then(setRows).catch((e) => setError(String(e)));
  }, [type]);

  return (
    <div className="space-y-3">
      {error && <Alert kind="error">{error}</Alert>}
      <select className={`${inputClass} max-w-xs`} value={type} onChange={(e) => setType(e.target.value)}>
        <option value="">كل الأنواع</option>
        {Object.entries(movementLabels).map(([k, v]) => <option key={k} value={k}>{v}</option>)}
      </select>
      <div className={`${cardClass} overflow-x-auto`}>
        <table className="w-full text-sm">
          <thead className="text-slate-500">
            <tr>
              <th className="p-3 text-start">التاريخ</th>
              <th className="p-3 text-start">النوع</th>
              <th className="p-3 text-start">الصنف</th>
              <th className="p-3 text-start">المخزن</th>
              <th className="p-3 text-start">الكمية</th>
              <th className="p-3 text-start">القيمة</th>
              <th className="p-3 text-start">ملاحظة</th>
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 && <tr><td colSpan={7} className="p-6 text-center text-slate-400">لا توجد حركات.</td></tr>}
            {rows.map((m) => (
              <tr key={m.id} className="border-t border-slate-100 dark:border-slate-700">
                <td className="p-3 whitespace-nowrap" dir="ltr">{m.createdAt.slice(0, 16).replace("T", " ")}</td>
                <td className="p-3">{movementLabels[m.movementType] ?? m.movementType}</td>
                <td className="p-3">{m.nameAr}{m.variantName ? ` — ${m.variantName}` : ""}</td>
                <td className="p-3">{m.warehouseName}</td>
                <td className="p-3" dir="ltr">{m.qtyDeltaMilli > 0 ? "+" : ""}{formatQty(m.qtyDeltaMilli)}</td>
                <td className="p-3" dir="ltr">{formatMoney(m.valueDeltaMinor)}</td>
                <td className="p-3 text-slate-500">{m.note}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
