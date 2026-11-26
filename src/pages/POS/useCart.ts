import { useCallback, useEffect, useRef, useState } from "react";
import { stepQty, qtyToMilli, milliToQty } from "../../lib/qty";
import { api, type CartInput, type CartQuote, type DiscountInput, type PaymentInput, type Sellable } from "../../services/api";

export interface CartLine {
  key: string;
  sellable: Sellable;
  unitId: string;
  qty: string;
  discount: DiscountInput | null;
}

export function toCartInput(lines: CartLine[], invoiceDiscount: DiscountInput | null, payments: PaymentInput[] = []): CartInput {
  return {
    items: lines.map((l) => ({
      productId: l.sellable.productId,
      variantId: l.sellable.variantId,
      unitId: l.unitId === l.sellable.baseUnitId ? null : l.unitId,
      qty: l.qty,
      discount: l.discount,
    })),
    invoiceDiscount,
    payments,
  };
}

let counter = 0;
const newKey = () => `l${Date.now()}-${counter++}`;

/** حالة السلة في الواجهة. كل الأرقام المالية تأتي من `price_cart` في Rust. */
export function useCart() {
  const [lines, setLines] = useState<CartLine[]>([]);
  const [invoiceDiscount, setInvoiceDiscount] = useState<DiscountInput | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [quote, setQuote] = useState<CartQuote | null>(null);
  const [quoteError, setQuoteError] = useState<string | null>(null);
  const seq = useRef(0);

  useEffect(() => {
    if (lines.length === 0) {
      setQuote(null);
      setQuoteError(null);
      return;
    }
    const mine = ++seq.current;
    api
      .priceCart(toCartInput(lines, invoiceDiscount))
      .then((q) => {
        if (mine !== seq.current) return;
        setQuote(q);
        setQuoteError(null);
      })
      .catch((e) => {
        if (mine === seq.current) setQuoteError(String(e));
      });
  }, [lines, invoiceDiscount]);

  const add = useCallback((s: Sellable) => {
    setLines((prev) => {
      const unitId = s.matchedUnitId ?? s.defaultUnitId;
      const existing = prev.find(
        (l) => l.sellable.productId === s.productId && l.sellable.variantId === s.variantId && l.unitId === unitId && !l.discount,
      );
      if (existing) {
        setSelected(existing.key);
        return prev.map((l) => (l.key === existing.key ? { ...l, qty: stepQty(l.qty, 1) } : l));
      }
      const key = newKey();
      setSelected(key);
      return [...prev, { key, sellable: s, unitId, qty: "1", discount: null }];
    });
  }, []);

  const patch = useCallback((key: string, p: Partial<CartLine>) => {
    setLines((prev) => prev.map((l) => (l.key === key ? { ...l, ...p } : l)));
  }, []);

  const remove = useCallback((key: string) => {
    setLines((prev) => {
      const i = prev.findIndex((l) => l.key === key);
      const next = prev.filter((l) => l.key !== key);
      setSelected(next[Math.min(i, next.length - 1)]?.key ?? null);
      return next;
    });
  }, []);

  const clear = useCallback(() => {
    setLines([]);
    setInvoiceDiscount(null);
    setSelected(null);
  }, []);

  const serialize = useCallback(() => JSON.stringify({ lines, invoiceDiscount }), [lines, invoiceDiscount]);

  const restore = useCallback((payload: string) => {
    const data = JSON.parse(payload) as { lines: CartLine[]; invoiceDiscount: DiscountInput | null };
    setLines(data.lines.map((l) => ({ ...l, key: newKey() })));
    setInvoiceDiscount(data.invoiceDiscount ?? null);
    setSelected(null);
  }, []);

  return { lines, invoiceDiscount, setInvoiceDiscount, selected, setSelected, quote, quoteError, add, patch, remove, clear, serialize, restore };
}

export { qtyToMilli, milliToQty };
