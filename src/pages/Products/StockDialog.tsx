import { useEffect, useState } from "react";
import Alert from "../../components/Alert";
import Modal from "../../components/Modal";
import { btnGhost, btnPrimary, inputClass } from "../../components/ui";
import UnitLineFields from "../../components/UnitLineFields";
import { buildChoices, lineForChoice, type UnitLineState } from "../../lib/bigUnit";
import { formatBigUnitStock, formatQty } from "../../lib/format";
import { api, type ProductDetail, type ProductListItem, type Unit } from "../../services/api";

/** رصيد افتتاحي وتسوية يدوية (زيادة بتكلفة، أو نقص بمتوسط التكلفة). التكلفة والرصيد يحسبهما Rust. */
export default function StockDialog({ product, onDone, onClose }: { product: ProductListItem; onDone: () => void; onClose: () => void }) {
  const [detail, setDetail] = useState<ProductDetail | null>(null);
  const [variantId, setVariantId] = useState("");
  const [direction, setDirection] = useState<"increase" | "decrease">("increase");
  const [units, setUnits] = useState<Unit[]>([]);
  const [line, setLine] = useState<UnitLineState | null>(null);
  const [note, setNote] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    Promise.all([api.getProduct(product.id), api.listUnits()]).then(([d, u]) => {
      setDetail(d);
      setUnits(u);
      const active = d.variants.filter((v) => v.isActive);
      if (active.length > 0) setVariantId(active[0].id);
      setLine(lineForChoice(buildChoices(d, u, active[0]?.id ?? null, false)[0], ""));
    }).catch((e) => setError(String(e)));
  }, [product.id]);

  const needsVariant = (detail?.variants.filter((v) => v.isActive).length ?? 0) > 0;
  const choices = detail ? buildChoices(detail, units, needsVariant ? variantId : null, false) : [];
  const choice = choices.find((c) => c.unitId === line?.unitId) ?? choices[0];
  const big = choices.find((c) => !c.isBase);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (!line || !choice) return;
    setBusy(true);
    setError(null);
    try {
      await api.adjustStock({
        productId: product.id,
        variantId: needsVariant ? variantId : null,
        direction,
        qty: line.qty,
        unitId: choice.isBase ? null : choice.unitId,
        factor: choice.isBase ? undefined : line.factor,
        unitCost: direction === "increase" && line.cost.trim() ? line.cost : undefined,
        salePrice: direction === "increase" && line.salePrice.trim() && line.salePrice.trim() !== line.origSalePrice ? line.salePrice : undefined,
        note,
      });
      onDone();
    } catch (err) {
      setError(String(err));
      setBusy(false);
    }
  }

  return (
    <Modal title={`رصيد المخزون: ${product.nameAr}`} onClose={onClose}>
      <form onSubmit={submit} className="space-y-4">
        {error && <Alert kind="error">{error}</Alert>}
        <div className="text-sm text-slate-500">الرصيد الحالي: {formatQty(product.stockQty)} {product.baseUnitName}{big ? ` — ${formatBigUnitStock(product.stockQty, { nameAr: big.nameAr, factorMicro: big.factor * 1_000_000 }, product.baseUnitName)}` : ""}</div>
        {needsVariant && detail && (
          <label className="block text-sm">المتغير
            <select className={`${inputClass} mt-1`} value={variantId} onChange={(e) => setVariantId(e.target.value)}>
              {detail.variants.filter((v) => v.isActive).map((v) => <option key={v.id} value={v.id}>{v.nameAr}</option>)}
            </select>
          </label>
        )}
        <div className="flex gap-4 text-sm">
          <label className="flex items-center gap-2"><input type="radio" checked={direction === "increase"} onChange={() => setDirection("increase")} /> إضافة رصيد</label>
          <label className="flex items-center gap-2"><input type="radio" checked={direction === "decrease"} onChange={() => setDirection("decrease")} /> خصم رصيد (تالف/عجز)</label>
        </div>
        {line && choices.length > 0 && (
          <UnitLineFields
            choices={choices}
            value={line}
            isVariant={needsVariant}
            showSalePrice={direction === "increase"}
            onChange={setLine}
          />
        )}
        <label className="block text-sm">ملاحظة
          <input className={`${inputClass} mt-1`} value={note} onChange={(e) => setNote(e.target.value)} />
        </label>
        <div className="flex justify-end gap-2">
          <button type="button" className={btnGhost} onClick={onClose}>إلغاء</button>
          <button type="submit" className={btnPrimary} disabled={busy}>حفظ</button>
        </div>
      </form>
    </Modal>
  );
}
