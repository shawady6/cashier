import { Search } from "lucide-react";
import { forwardRef, useEffect, useImperativeHandle, useRef, useState } from "react";
import { inputClass } from "../../components/ui";
import { formatBigUnitStock, formatMoney, formatQty } from "../../lib/format";
import { api, type Sellable } from "../../services/api";

export interface SearchBoxHandle {
  focus: () => void;
}

/** حقل واحد للبحث والماسح: الماسح يكتب الباركود ثم Enter فيُضاف المنتج مباشرة. */
const SearchBox = forwardRef<SearchBoxHandle, { onPick: (s: Sellable) => void; onMessage: (m: string) => void }>(
  function SearchBox({ onPick, onMessage }, ref) {
    const [text, setText] = useState("");
    const [results, setResults] = useState<Sellable[]>([]);
    const [active, setActive] = useState(0);
    const inputRef = useRef<HTMLInputElement>(null);
    useImperativeHandle(ref, () => ({ focus: () => inputRef.current?.focus() }));

    useEffect(() => {
      const q = text.trim();
      if (!q) {
        setResults([]);
        return;
      }
      const t = setTimeout(() => {
        api.searchSellables(q).then((r) => { setResults(r); setActive(0); }).catch((e) => onMessage(String(e)));
      }, 120);
      return () => clearTimeout(t);
    }, [text, onMessage]);

    function pick(s: Sellable) {
      onPick(s);
      setText("");
      setResults([]);
      inputRef.current?.focus();
    }

    async function onEnter() {
      const q = text.trim();
      if (!q) return;
      // بحث جديد وقت Enter: الماسح أسرع من تأخير البحث المباشر
      try {
        const fresh = await api.searchSellables(q);
        if (fresh.length === 0) {
          onMessage(`لا يوجد منتج مطابق لـ «${q}»`);
          return;
        }
        const exact = fresh.find((s) => s.barcode === q || s.sku === q || s.matchedUnitId !== null);
        const chosen = exact ?? (fresh.length === 1 ? fresh[0] : results[active] ?? fresh[0]);
        pick(chosen);
      } catch (e) {
        onMessage(String(e));
      }
    }

    return (
      <div className="relative">
        <Search size={18} className="pointer-events-none absolute start-3 top-3 text-slate-400" />
        <input
          ref={inputRef}
          autoFocus
          value={text}
          dir="auto"
          aria-label="بحث أو مسح باركود"
          placeholder="ابحث بالاسم أو SKU، أو امسح الباركود…"
          className={`${inputClass} ps-10 py-3 text-base`}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") { e.preventDefault(); void onEnter(); }
            else if (e.key === "ArrowDown") { e.preventDefault(); setActive((a) => Math.min(a + 1, results.length - 1)); }
            else if (e.key === "ArrowUp") { e.preventDefault(); setActive((a) => Math.max(a - 1, 0)); }
            else if (e.key === "Escape") { setText(""); setResults([]); }
          }}
        />
        {results.length > 0 && (
          <ul role="listbox" className="absolute z-20 mt-1 max-h-96 w-full overflow-y-auto rounded-lg border border-slate-200 bg-white shadow-lg dark:border-slate-600 dark:bg-slate-800">
            {results.map((s, i) => (
              <li
                key={`${s.productId}-${s.variantId}`}
                role="option"
                aria-selected={i === active}
                onMouseEnter={() => setActive(i)}
                onMouseDown={(e) => { e.preventDefault(); pick(s); }}
                className={`flex cursor-pointer items-center justify-between px-4 py-2 text-sm ${i === active ? "bg-teal-50 dark:bg-teal-900/40" : ""}`}
              >
                <div>
                  <div className="font-medium">{s.nameAr}{s.variantName ? ` - ${s.variantName}` : ""}</div>
                  <div className="text-xs text-slate-500" dir="ltr">{[s.sku, s.barcode].filter(Boolean).join(" · ")}</div>
                </div>
                <div className="text-end">
                  <div className="font-semibold">{formatMoney(s.salePrice)}</div>
                  <div className={`text-xs ${s.stockQty <= 0 ? "text-red-600" : "text-slate-500"}`}>
                    المتاح: {formatQty(s.stockQty)} {s.baseUnitName}
                  </div>
                  {formatBigUnitStock(s.stockQty, s.units[1], s.baseUnitName) && (
                    <div className="text-xs text-slate-500">{formatBigUnitStock(s.stockQty, s.units[1], s.baseUnitName)}</div>
                  )}
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    );
  },
);

export default SearchBox;
