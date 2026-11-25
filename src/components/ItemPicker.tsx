import { useEffect, useState } from "react";
import { api, type Sellable } from "../services/api";
import { inputClass } from "./ui";

/** بحث عن صنف (اسم/باركود/SKU) عبر Rust، ثم اختياره. */
export default function ItemPicker({ onPick, placeholder }: { onPick: (s: Sellable) => void; placeholder?: string }) {
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<Sellable[]>([]);

  useEffect(() => {
    if (!q.trim()) {
      setHits([]);
      return;
    }
    let alive = true;
    const t = setTimeout(() => {
      api.searchSellables(q).then((r) => alive && setHits(r)).catch(() => alive && setHits([]));
    }, 200);
    return () => {
      alive = false;
      clearTimeout(t);
    };
  }, [q]);

  return (
    <div className="relative">
      <input
        className={inputClass}
        value={q}
        placeholder={placeholder ?? "ابحث بالاسم أو الباركود..."}
        onChange={(e) => setQ(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && hits.length > 0) {
            e.preventDefault();
            onPick(hits[0]);
            setQ("");
          }
        }}
      />
      {hits.length > 0 && (
        <ul className="absolute z-20 mt-1 max-h-64 w-full overflow-y-auto rounded-lg border border-slate-200 bg-white shadow-lg dark:border-slate-600 dark:bg-slate-800">
          {hits.map((h) => (
            <li key={`${h.productId}-${h.variantId ?? ""}`}>
              <button
                type="button"
                className="w-full px-3 py-2 text-start text-sm hover:bg-slate-100 dark:hover:bg-slate-700"
                onClick={() => {
                  onPick(h);
                  setQ("");
                }}
              >
                {h.nameAr}
                {h.variantName ? ` — ${h.variantName}` : ""}
                <span className="ms-2 text-xs text-slate-400">{h.sku ?? h.barcode ?? ""}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
