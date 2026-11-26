import { Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import Alert from "../../components/Alert";
import ItemPicker from "../../components/ItemPicker";
import Modal from "../../components/Modal";
import UnitLineFields from "../../components/UnitLineFields";
import { btnGhost, btnPrimary } from "../../components/ui";
import { buildChoices, lineForChoice, type UnitChoice, type UnitLineState } from "../../lib/bigUnit";
import { api, type AdjustInput, type Sellable, type Unit } from "../../services/api";

interface Line {
  key: string;
  productId: string;
  variantId: string | null;
  name: string;
  choices: UnitChoice[];
  state: UnitLineState;
}

let counter = 0;

/** رصيد افتتاحي لعدة أصناف دفعة واحدة بالوحدة الكبيرة أو القطعة، يُحفظ كله أو لا يُحفظ شيء. */
export default function OpeningStock({ onClose, onDone }: { onClose: () => void; onDone: () => void }) {
  const [units, setUnits] = useState<Unit[]>([]);
  const [lines, setLines] = useState<Line[]>([]);
  const [confirming, setConfirming] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.listUnits().then(setUnits).catch((e) => setError(String(e)));
  }, []);

  async function addLine(sel: Sellable) {
    try {
      const det = await api.getProduct(sel.productId);
      const choices = buildChoices(det, units, sel.variantId, false);
      setDirty(true);
      setLines((ls) => [...ls, {
        key: `o${counter++}`, productId: sel.productId, variantId: sel.variantId,
        name: sel.nameAr + (sel.variantName ? ` — ${sel.variantName}` : ""), choices, state: lineForChoice(choices[0]),
      }]);
    } catch (e) {
      setError(String(e));
    }
  }

  function close() {
    if (!dirty || window.confirm("في أصناف لم تُحفظ. تخرج بدونها؟")) onClose();
  }

  async function save() {
    setBusy(true);
    setError(null);
    try {
      const inputs: AdjustInput[] = lines.map((l) => {
        const c = l.choices.find((x) => x.unitId === l.state.unitId) ?? l.choices[0];
        const price = l.state.salePrice.trim();
        return {
          productId: l.productId,
          variantId: l.variantId,
          direction: "increase",
          qty: l.state.qty,
          unitId: c.isBase ? null : c.unitId,
          factor: c.isBase ? undefined : l.state.factor,
          unitCost: l.state.cost,
          salePrice: price !== "" && price !== l.state.origSalePrice ? price : undefined,
          note: "رصيد افتتاحي",
        };
      });
      await api.adjustStockMany(inputs);
      setDirty(false);
      onDone();
    } catch (e) {
      setError(String(e));
      setBusy(false);
      setConfirming(false);
    }
  }

  return (
    <Modal title="رصيد افتتاحي" onClose={close} wide>
      <div className="space-y-3">
        {error && <Alert kind="error">{error}</Alert>}
        <ItemPicker onPick={(sel) => void addLine(sel)} />
        <div className="space-y-3">
          {lines.length === 0 && <p className="p-4 text-center text-sm text-slate-400">لا توجد أصناف بعد.</p>}
          {lines.map((l) => (
            <div key={l.key} className="rounded-lg border border-slate-200 p-3 dark:border-slate-700">
              <div className="mb-2 flex items-center justify-between text-sm font-semibold">
                <span>{l.name}</span>
                <button aria-label="حذف" onClick={() => { setDirty(true); setLines((ls) => ls.filter((x) => x.key !== l.key)); }}><Trash2 size={16} /></button>
              </div>
              <UnitLineFields
                choices={l.choices}
                value={l.state}
                isVariant={l.variantId !== null}
                onChange={(st) => { setDirty(true); setLines((ls) => ls.map((x) => (x.key === l.key ? { ...x, state: st } : x))); }}
              />
            </div>
          ))}
        </div>
        {confirming ? (
          <div className="flex items-center justify-between rounded-lg border border-teal-300 bg-teal-50 p-3 text-sm dark:border-teal-700 dark:bg-teal-950/30">
            <span>اعتماد الرصيد الافتتاحي لـ {lines.length} صنف؟</span>
            <span className="flex gap-2">
              <button className={btnGhost} disabled={busy} onClick={() => setConfirming(false)}>رجوع</button>
              <button className={btnPrimary} disabled={busy} onClick={() => void save()}>تأكيد الاعتماد</button>
            </span>
          </div>
        ) : (
          <div className="flex justify-end gap-2">
            <button className={btnGhost} onClick={close}>إلغاء</button>
            <button className={btnPrimary} disabled={lines.length === 0} onClick={() => setConfirming(true)}>اعتماد</button>
          </div>
        )}
      </div>
    </Modal>
  );
}
