import { fmt2, lineForChoice, previewLine, type UnitChoice, type UnitLineState } from "../lib/bigUnit";
import { inputClass } from "./ui";

/** حقول وحدة السطر: الوحدة، الكمية، عدد القطع (للوحدة الكبيرة)، التكلفة، سعر البيع، وملخص حي وتحذير. */
export default function UnitLineFields({
  choices,
  value,
  onChange,
  isVariant,
  showSalePrice = true,
}: {
  choices: UnitChoice[];
  value: UnitLineState;
  onChange: (next: UnitLineState) => void;
  isVariant: boolean;
  showSalePrice?: boolean;
}) {
  const choice = choices.find((c) => c.unitId === value.unitId) ?? choices[0];
  const base = { nameAr: choice.baseName };
  const pv = previewLine(choice, value, isVariant);
  const set = (p: Partial<UnitLineState>) => onChange({ ...value, ...p });
  const showSale = showSalePrice && !(isVariant && !choice.isBase);

  return (
    <div className="space-y-2">
      <div className="grid gap-2 sm:grid-cols-5">
        <label className="text-xs text-slate-500">الوحدة
          <select
            className={`${inputClass} mt-1`}
            value={value.unitId}
            onChange={(e) => {
              const c = choices.find((x) => x.unitId === e.target.value);
              if (c) onChange({ ...lineForChoice(c, value.qty) });
            }}
          >
            {choices.map((c) => <option key={c.unitId} value={c.unitId}>{c.nameAr}</option>)}
          </select>
        </label>
        <label className="text-xs text-slate-500">الكمية
          <input className={`${inputClass} mt-1`} dir="ltr" inputMode="decimal" value={value.qty} onChange={(e) => set({ qty: e.target.value })} />
        </label>
        {!choice.isBase && (
          <label className="text-xs text-slate-500">عدد {base.nameAr} فيها
            <input className={`${inputClass} mt-1`} dir="ltr" inputMode="numeric" value={value.factor} onChange={(e) => set({ factor: e.target.value })} />
          </label>
        )}
        <label className="text-xs text-slate-500">تكلفة {choice.nameAr} (ج.م)
          <input className={`${inputClass} mt-1`} dir="ltr" inputMode="decimal" value={value.cost} onChange={(e) => set({ cost: e.target.value })} />
        </label>
        {showSale && (
          <label className="text-xs text-slate-500">سعر بيع {choice.nameAr} (ج.م)
            <input className={`${inputClass} mt-1`} dir="ltr" inputMode="decimal" value={value.salePrice} onChange={(e) => set({ salePrice: e.target.value })} />
          </label>
        )}
      </div>
      {pv && (
        <p className="text-xs text-slate-600 dark:text-slate-300">
          {choice.isBase
            ? `${value.qty} ${choice.nameAr} · تكلفة القطعة ${fmt2(pv.costPerPiece)}`
            : `${value.qty} ${choice.nameAr} × ${value.factor} = ${pv.basePieces} ${base.nameAr}، تكلفة ${base.nameAr} ${fmt2(pv.costPerPiece)}`}
          {showSale && value.salePrice.trim() ? `، سعر بيع ${choice.nameAr} ${value.salePrice}` : ""}
        </p>
      )}
      {showSale && pv?.below && <p className="text-xs font-semibold text-amber-600">تنبيه: سعر البيع أقل من سعر الشراء.</p>}
      {showSale && pv?.priceChanges && <p className="text-xs font-semibold text-teal-700">سيتم تحديث سعر البيع عند الاستلام.</p>}
    </div>
  );
}
