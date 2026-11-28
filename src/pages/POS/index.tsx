import { Minus, PauseCircle, PercentCircle, Plus, Printer, RotateCcw, Trash2, Wallet, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import Alert from "../../components/Alert";
import Modal from "../../components/Modal";
import ReceiptDialog from "../../components/ReceiptDialog";
import { btnGhost, btnPrimary, cardClass, inputClass } from "../../components/ui";
import { formatBigUnitStock, formatMoney, formatQty } from "../../lib/format";
import { isThermal, printSaleThermal, shouldOpenDrawer } from "../../lib/printing";
import { scannerConfig, useGlobalScan } from "../../lib/scanner";
import { receiptSettings } from "../../components/Receipt";
import { parseShortcuts } from "../../lib/shortcuts";
import { stepQty } from "../../lib/qty";
import { api, type CheckoutResult } from "../../services/api";
import { useSettingsStore } from "../../store/settingsStore";
import DiscountDialog from "./DiscountDialog";
import PaymentDialog from "./PaymentDialog";
import SearchBox, { type SearchBoxHandle } from "./SearchBox";
import SuspendedDialog from "./SuspendedDialog";
import { toCartInput, useCart } from "./useCart";

type Dialog = null | "pay" | "resume" | "invoiceDiscount" | { lineDiscount: string };

export default function PosPage() {
  const shortcutsRaw = useSettingsStore((s) => s.settings.pos_shortcuts);
  const keys = useMemo(() => parseShortcuts(shortcutsRaw), [shortcutsRaw]);
  const cart = useCart();
  const [dialog, setDialog] = useState<Dialog>(null);
  const [done, setDone] = useState<CheckoutResult | null>(null);
  const [receiptFor, setReceiptFor] = useState<string | null>(null);
  const [message, setMessage] = useState<{ kind: "error" | "ok"; text: string } | null>(null);
  const searchRef = useRef<SearchBoxHandle>(null);
  const allSettings = useSettingsStore((st) => st.settings);
  useGlobalScan(scannerConfig(allSettings), !dialog && !done && !receiptFor, (code) => searchRef.current?.scan(code));

  const notify = useCallback((text: string, kind: "error" | "ok" = "error") => setMessage({ kind, text }), []);

  /** بعد البيع: طباعة حرارية تلقائية وفتح الدُرج حسب الإعدادات. فشل الجهاز لا يلغي البيع. */
  async function afterSale(r: CheckoutResult) {
    if (r.duplicate) return;
    const thermal = isThermal(allSettings);
    const wantsPrint = thermal && allSettings.print_auto === "1";
    if (!wantsPrint && allSettings.drawer_mode !== "cash" && allSettings.drawer_mode !== "always") return;
    try {
      const sale = await api.getSale(r.saleId);
      const drawer = !!allSettings.printer_name && shouldOpenDrawer(allSettings, sale.payments.map((p) => p.method));
      if (wantsPrint) await printSaleThermal(sale, receiptSettings(allSettings), drawer);
      else if (drawer) await api.openCashDrawer();
    } catch (e) {
      notify(`تم البيع، لكن تعذر تشغيل الطابعة أو الدُرج: ${String(e)}`);
    }
  }
  const focusSearch = () => setTimeout(() => searchRef.current?.focus(), 0);

  const baseCart = useMemo(() => toCartInput(cart.lines, cart.invoiceDiscount), [cart.lines, cart.invoiceDiscount]);
  const hasItems = cart.lines.length > 0;

  async function suspend() {
    if (!hasItems) return;
    try {
      const label = `فاتورة ${new Date().toLocaleTimeString("ar-EG", { hour: "2-digit", minute: "2-digit" })}`;
      await api.suspendSale(label, cart.serialize());
      cart.clear();
      notify("تم تعليق الفاتورة", "ok");
      focusSearch();
    } catch (e) {
      notify(String(e));
    }
  }

  function openResume() {
    if (hasItems) return notify("علّق الفاتورة الحالية أو أنهِها قبل استرجاع فاتورة معلّقة");
    setDialog("resume");
  }

  function openPay() {
    if (!hasItems) return notify("السلة فارغة");
    if (!cart.quote || cart.quoteError) return notify(cart.quoteError ?? "جارٍ حساب الفاتورة…");
    setDialog("pay");
  }

  // اختصارات لوحة المفاتيح (قابلة للتخصيص من الإعدادات). Esc تُغلق النوافذ من Modal نفسها.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      const typing = target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.tagName === "SELECT";
      if (dialog) return;
      if (e.key === keys.search) { e.preventDefault(); searchRef.current?.focus(); }
      else if (e.key === keys.pay) { e.preventDefault(); openPay(); }
      else if (e.key === keys.suspend) { e.preventDefault(); void suspend(); }
      else if (e.key === keys.resume) { e.preventDefault(); openResume(); }
      else if (e.key === keys.remove && !typing && cart.selected) { e.preventDefault(); cart.remove(cart.selected); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const lineDiscountKey = typeof dialog === "object" && dialog ? dialog.lineDiscount : null;
  const lineForDiscount = cart.lines.find((l) => l.key === lineDiscountKey);

  return (
    <div className="grid h-[calc(100vh-8.5rem)] gap-4 lg:grid-cols-[1fr_22rem]">
      <div className="flex min-h-0 flex-col gap-3">
        <SearchBox ref={searchRef} onPick={(s) => { cart.add(s); setMessage(null); }} onMessage={notify} />
        {message && <Alert kind={message.kind}>{message.text}</Alert>}
        {cart.quoteError && <Alert kind="error">{cart.quoteError}</Alert>}

        <div className={`${cardClass} min-h-0 flex-1 overflow-y-auto`}>
          <table className="w-full text-sm">
            <thead className="sticky top-0 bg-slate-50 text-slate-500 dark:bg-slate-900">
              <tr>
                <th className="px-3 py-2 text-start">المنتج</th>
                <th className="px-3 py-2 text-start">الوحدة</th>
                <th className="px-3 py-2 text-center">الكمية</th>
                <th className="px-3 py-2 text-start">السعر</th>
                <th className="px-3 py-2 text-start">الخصم</th>
                <th className="px-3 py-2 text-start">الإجمالي</th>
                <th className="w-20" />
              </tr>
            </thead>
            <tbody>
              {!hasItems && (
                <tr><td colSpan={7} className="px-4 py-16 text-center text-slate-500">
                  السلة فارغة. امسح الباركود أو ابحث عن منتج ({keys.search}).
                </td></tr>
              )}
              {cart.lines.map((l, i) => {
                const q = cart.quote?.lines[i];
                return (
                  <tr
                    key={l.key}
                    onClick={() => cart.setSelected(l.key)}
                    className={`border-t border-slate-100 dark:border-slate-700 ${cart.selected === l.key ? "bg-teal-50 dark:bg-teal-900/30" : ""}`}
                  >
                    <td className="px-3 py-2">
                      <div className="font-medium">{l.sellable.nameAr}{l.sellable.variantName ? ` - ${l.sellable.variantName}` : ""}</div>
                      <div className="text-xs text-slate-500">المتاح: {formatQty(l.sellable.stockQty)} {l.sellable.baseUnitName}{formatBigUnitStock(l.sellable.stockQty, l.sellable.units.find((u) => u.unitId !== l.sellable.baseUnitId), l.sellable.baseUnitName) ? ` (${formatBigUnitStock(l.sellable.stockQty, l.sellable.units.find((u) => u.unitId !== l.sellable.baseUnitId), l.sellable.baseUnitName)})` : ""}</div>
                    </td>
                    <td className="px-3 py-2">
                      {l.sellable.units.length > 1 ? (
                        <select
                          className={`${inputClass} w-28 py-1`}
                          value={l.unitId}
                          aria-label="وحدة البيع"
                          onChange={(e) => cart.patch(l.key, { unitId: e.target.value })}
                        >
                          {l.sellable.units.map((u) => <option key={u.unitId} value={u.unitId}>{u.nameAr}</option>)}
                        </select>
                      ) : (l.sellable.units[0]?.nameAr ?? l.sellable.baseUnitName)}
                    </td>
                    <td className="px-3 py-2">
                      <div className="flex items-center justify-center gap-1">
                        <button className={btnGhost} aria-label="إنقاص" onClick={() => cart.patch(l.key, { qty: stepQty(l.qty, -1) })}><Minus size={14} /></button>
                        <input
                          dir="ltr"
                          inputMode="decimal"
                          aria-label="الكمية"
                          className={`${inputClass} w-20 text-center`}
                          value={l.qty}
                          onFocus={() => cart.setSelected(l.key)}
                          onChange={(e) => cart.patch(l.key, { qty: e.target.value })}
                        />
                        <button className={btnGhost} aria-label="زيادة" onClick={() => cart.patch(l.key, { qty: stepQty(l.qty, 1) })}><Plus size={14} /></button>
                      </div>
                    </td>
                    <td className="px-3 py-2">{q ? formatMoney(q.unitPrice) : "…"}</td>
                    <td className="px-3 py-2">
                      <button className="text-teal-700 underline" onClick={() => setDialog({ lineDiscount: l.key })}>
                        {q && q.discount > 0 ? `-${formatMoney(q.discount)}` : "إضافة"}
                      </button>
                    </td>
                    <td className="px-3 py-2 font-semibold">{q ? formatMoney(q.gross - q.discount) : "…"}</td>
                    <td className="px-2">
                      <button className={`${btnGhost} text-red-600`} aria-label="حذف البند" onClick={() => cart.remove(l.key)}><Trash2 size={14} /></button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>

      <aside className={`${cardClass} flex flex-col gap-3 p-4`}>
        <h3 className="font-semibold">ملخص الفاتورة</h3>
        <dl className="space-y-2 text-sm">
          <div className="flex justify-between"><dt>الإجمالي قبل الخصم</dt><dd>{formatMoney(cart.quote?.subtotal ?? 0)}</dd></div>
          <div className="flex justify-between"><dt>خصم البنود</dt><dd>-{formatMoney(cart.quote?.itemDiscount ?? 0)}</dd></div>
          <div className="flex justify-between">
            <dt>
              خصم الفاتورة{" "}
              <button className="text-teal-700 underline disabled:opacity-40" disabled={!hasItems} onClick={() => setDialog("invoiceDiscount")}>تعديل</button>
            </dt>
            <dd>-{formatMoney(cart.quote?.invoiceDiscount ?? 0)}</dd>
          </div>
          {(cart.quote?.tax ?? 0) > 0 && (
            <div className="flex justify-between text-slate-500"><dt>منها ضريبة</dt><dd>{formatMoney(cart.quote?.tax ?? 0)}</dd></div>
          )}
        </dl>
        <div className="rounded-xl bg-navy-900 p-4 text-white">
          <div className="text-sm opacity-80">الإجمالي</div>
          <div className="text-3xl font-bold">{formatMoney(cart.quote?.total ?? 0)}</div>
        </div>
        <button className={`${btnPrimary} justify-center py-3 text-base`} disabled={!hasItems} onClick={openPay}>
          <Wallet size={18} /> الدفع ({keys.pay})
        </button>
        <div className="grid grid-cols-2 gap-2">
          <button className={`${btnGhost} justify-center`} disabled={!hasItems} onClick={suspend}><PauseCircle size={16} /> تعليق ({keys.suspend})</button>
          <button className={`${btnGhost} justify-center`} onClick={openResume}><RotateCcw size={16} /> استرجاع ({keys.resume})</button>
        </div>
        <button
          className={`${btnGhost} justify-center text-red-600`}
          disabled={!hasItems}
          onClick={() => { if (window.confirm("إلغاء الفاتورة الحالية؟")) { cart.clear(); focusSearch(); } }}
        >
          <X size={16} /> إلغاء الفاتورة
        </button>
        <p className="mt-auto text-xs text-slate-500">
          <PercentCircle size={12} className="inline" /> حذف البند المحدد: {keys.remove} · الإغلاق: Esc
        </p>
      </aside>

      {dialog === "pay" && cart.quote && (
        <PaymentDialog
          baseCart={baseCart}
          total={cart.quote.total}
          onClose={() => { setDialog(null); focusSearch(); }}
          onDone={(r) => { setDialog(null); setDone(r); cart.clear(); void afterSale(r); }}
        />
      )}
      {dialog === "resume" && (
        <SuspendedDialog
          onClose={() => { setDialog(null); focusSearch(); }}
          onResume={(s) => {
            try { cart.restore(s.payload); setDialog(null); focusSearch(); } catch { notify("تعذّر قراءة الفاتورة المعلّقة"); setDialog(null); }
          }}
        />
      )}
      {dialog === "invoiceDiscount" && (
        <DiscountDialog
          title="خصم على الفاتورة"
          initial={cart.invoiceDiscount}
          onClose={() => setDialog(null)}
          onSave={(d) => { cart.setInvoiceDiscount(d); setDialog(null); focusSearch(); }}
        />
      )}
      {lineForDiscount && (
        <DiscountDialog
          title={`خصم على: ${lineForDiscount.sellable.nameAr}`}
          initial={lineForDiscount.discount}
          onClose={() => setDialog(null)}
          onSave={(d) => { cart.patch(lineForDiscount.key, { discount: d }); setDialog(null); focusSearch(); }}
        />
      )}

      {done && (
        <Modal title="تم البيع بنجاح" onClose={() => { setDone(null); focusSearch(); }}>
          <div className="space-y-4 text-center">
            <div className="text-sm text-slate-500">رقم الفاتورة <span dir="ltr" className="font-semibold">{done.invoiceNo}</span></div>
            <div className="text-2xl font-bold">{formatMoney(done.total)}</div>
            {done.change > 0 && (
              <div className="rounded-xl bg-teal-50 p-4 text-teal-800">
                <div className="text-sm">الباقي للعميل</div>
                <div className="text-3xl font-bold">{formatMoney(done.change)}</div>
              </div>
            )}
            <div className="flex justify-center gap-2">
              <button className={btnGhost} onClick={() => setReceiptFor(done.saleId)}><Printer size={16} /> طباعة الإيصال</button>
              <button autoFocus className={btnPrimary} onClick={() => { setDone(null); focusSearch(); }}>فاتورة جديدة</button>
            </div>
          </div>
        </Modal>
      )}
      {receiptFor && <ReceiptDialog saleId={receiptFor} onClose={() => setReceiptFor(null)} />}
    </div>
  );
}
