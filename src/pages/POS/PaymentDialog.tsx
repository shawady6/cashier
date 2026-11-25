import { useEffect, useRef, useState } from "react";
import Alert from "../../components/Alert";
import Modal from "../../components/Modal";
import { btnGhost, btnPrimary, inputClass } from "../../components/ui";
import { formatMoney, minorToInput } from "../../lib/format";
import { api, type CartInput, type CartQuote, type CheckoutResult, type Customer } from "../../services/api";

/**
 * الدفع: الحقول نصوص فقط. المدفوع والباقي والفكة تُحسب في Rust عبر price_cart،
 * وإتمام البيع يستخدم معرّف طلب ثابتًا فلا تتكرر الفاتورة لو ضُغط الزر مرتين.
 */
export default function PaymentDialog({
  baseCart,
  total,
  onDone,
  onClose,
}: {
  baseCart: CartInput;
  total: number;
  onDone: (r: CheckoutResult) => void;
  onClose: () => void;
}) {
  const [cash, setCash] = useState("");
  const [card, setCard] = useState("");
  const [quote, setQuote] = useState<CartQuote | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [customers, setCustomers] = useState<Customer[]>([]);
  const [customerId, setCustomerId] = useState("");
  const requestId = useRef(crypto.randomUUID());
  const seq = useRef(0);

  const payments = [
    ...(cash.trim() ? [{ method: "cash" as const, amount: cash }] : []),
    ...(card.trim() ? [{ method: "card" as const, amount: card }] : []),
  ];

  useEffect(() => {
    api.listCustomers().then((c) => setCustomers(c.filter((x) => x.isActive))).catch(() => undefined);
  }, []);

  useEffect(() => {
    const mine = ++seq.current;
    api
      .priceCart({ ...baseCart, payments, customerId: customerId || null })
      .then((q) => { if (mine === seq.current) { setQuote(q); setError(null); } })
      .catch((e) => { if (mine === seq.current) setError(String(e)); });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cash, card, baseCart, customerId]);

  async function confirm() {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      const r = await api.checkout({ ...baseCart, payments, customerId: customerId || null, clientRequestId: requestId.current });
      onDone(r);
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  }

  const credit = quote !== null && quote.remaining > 0;
  const ready = quote !== null && (quote.remaining === 0 || customerId !== "") && !error;
  const chosen = customers.find((c) => c.id === customerId);

  return (
    <Modal title="الدفع" onClose={busy ? () => {} : onClose}>
      <form onSubmit={(e) => { e.preventDefault(); if (ready) void confirm(); }} className="space-y-4">
        <div className="rounded-xl bg-navy-900 p-4 text-center text-white">
          <div className="text-sm opacity-80">الإجمالي المطلوب</div>
          <div className="text-3xl font-bold">{formatMoney(total)}</div>
        </div>
        {error && <Alert kind="error">{error}</Alert>}
        <div className="grid grid-cols-2 gap-3">
          <label className="text-sm">
            نقدًا
            <input autoFocus inputMode="decimal" className={`${inputClass} mt-1 text-lg`} value={cash} onChange={(e) => setCash(e.target.value)} />
          </label>
          <label className="text-sm">
            كارت
            <input inputMode="decimal" className={`${inputClass} mt-1 text-lg`} value={card} onChange={(e) => setCard(e.target.value)} />
          </label>
        </div>
        <div className="flex gap-2">
          <button type="button" className={btnGhost} onClick={() => { setCash(minorToInput(total)); setCard(""); }}>
            الكل نقدًا
          </button>
          <button type="button" className={btnGhost} onClick={() => { setCard(minorToInput(total)); setCash(""); }}>
            الكل كارت
          </button>
        </div>
        <div className="grid grid-cols-3 gap-2 rounded-lg bg-slate-50 p-3 text-center text-sm dark:bg-slate-900/40">
          <div><div className="text-slate-500">المدفوع</div><div className="font-semibold">{formatMoney(quote?.paid ?? 0)}</div></div>
          <div><div className="text-slate-500">المتبقي</div><div className={`font-semibold ${quote && quote.remaining > 0 && !customerId ? "text-red-600" : ""}`}>{formatMoney(quote?.remaining ?? total)}</div></div>
          <div><div className="text-slate-500">الباقي للعميل</div><div className="text-lg font-bold text-teal-700">{formatMoney(quote?.change ?? 0)}</div></div>
        </div>
        <label className="block text-sm">
          العميل {credit ? "(مطلوب للبيع الآجل)" : "(اختياري)"}
          <select className={`${inputClass} mt-1`} value={customerId} onChange={(e) => setCustomerId(e.target.value)}>
            <option value="">بدون عميل</option>
            {customers.map((c) => <option key={c.id} value={c.id}>{c.nameAr}</option>)}
          </select>
        </label>
        {chosen && (
          <p className="text-xs text-slate-500">
            مستحق عليه الآن: {formatMoney(chosen.balanceMinor)}
            {chosen.creditLimitMinor > 0 ? ` · الحد الائتماني ${formatMoney(chosen.creditLimitMinor)}` : ""}
          </p>
        )}
        {credit && customerId !== "" && <Alert kind="ok">سيُسجَّل {formatMoney(quote?.remaining ?? 0)} آجل على العميل.</Alert>}
        <div className="flex justify-end gap-2">
          <button type="button" className={btnGhost} onClick={onClose} disabled={busy}>رجوع</button>
          <button type="submit" className={btnPrimary} disabled={!ready || busy}>
            {busy ? "جارٍ الحفظ…" : "إتمام البيع"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
