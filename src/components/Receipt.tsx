import { formatMoney, formatQty } from "../lib/format";
import type { SaleDetail } from "../services/api";

export interface ReceiptSettings {
  storeName: string;
  address: string;
  phone: string;
  footer: string;
  paperWidth: "58" | "80";
}

export function receiptSettings(s: Record<string, string>): ReceiptSettings {
  return {
    storeName: s.store_name ?? "",
    address: s.store_address ?? "",
    phone: s.store_phone ?? "",
    footer: s.receipt_footer ?? "",
    paperWidth: s.receipt_paper_width === "58" ? "58" : "80",
  };
}

const methodLabel: Record<string, string> = { cash: "نقدًا", card: "كارت" };

/** إيصال للمعاينة والطباعة عبر طباعة ويندوز (58 أو 80 مم). كل الأرقام جاهزة من Rust. */
export default function Receipt({ sale, settings }: { sale: SaleDetail; settings: ReceiptSettings }) {
  const mm = settings.paperWidth;
  const date = new Date(sale.createdAt).toLocaleString("ar-EG");
  return (
    <>
      <style>{`@media print { @page { size: ${mm}mm auto; margin: 0 } }`}</style>
      <div
        className="receipt-print mx-auto bg-white p-3 text-black"
        style={{ width: `${mm}mm`, fontSize: mm === "58" ? "10px" : "12px", lineHeight: 1.5 }}
        dir="rtl"
      >
        <div className="text-center">
          <div className="text-base font-bold">{settings.storeName || "المتجر"}</div>
          {settings.address && <div>{settings.address}</div>}
          {settings.phone && <div dir="ltr">{settings.phone}</div>}
        </div>
        <hr className="my-1 border-dashed border-black" />
        <div className="flex justify-between"><span>فاتورة</span><span dir="ltr">{sale.invoiceNo}</span></div>
        <div className="flex justify-between"><span>التاريخ</span><span>{date}</span></div>
        <hr className="my-1 border-dashed border-black" />
        {sale.items.map((i) => (
          <div key={i.id} className="mb-1">
            <div>{i.nameAr}{i.variantName ? ` - ${i.variantName}` : ""}</div>
            <div className="flex justify-between">
              <span>{formatQty(i.qtyMilli)} {i.unitName} × {formatMoney(i.unitPriceMinor)}</span>
              <span>{formatMoney(i.grossMinor)}</span>
            </div>
            {i.discountMinor > 0 && (
              <div className="flex justify-between"><span>خصم</span><span>-{formatMoney(i.discountMinor)}</span></div>
            )}
          </div>
        ))}
        <hr className="my-1 border-dashed border-black" />
        <div className="flex justify-between"><span>الإجمالي قبل الخصم</span><span>{formatMoney(sale.subtotalMinor)}</span></div>
        {sale.itemDiscountMinor + sale.invoiceDiscountMinor > 0 && (
          <div className="flex justify-between">
            <span>إجمالي الخصم</span><span>-{formatMoney(sale.itemDiscountMinor + sale.invoiceDiscountMinor)}</span>
          </div>
        )}
        {sale.taxMinor > 0 && (
          <div className="flex justify-between"><span>شامل الضريبة</span><span>{formatMoney(sale.taxMinor)}</span></div>
        )}
        <div className="flex justify-between text-base font-bold"><span>الإجمالي</span><span>{formatMoney(sale.totalMinor)}</span></div>
        <hr className="my-1 border-dashed border-black" />
        {sale.payments.map((p, idx) => (
          <div key={idx} className="flex justify-between">
            <span>{methodLabel[p.method] ?? p.method}</span><span>{formatMoney(p.amountMinor)}</span>
          </div>
        ))}
        {sale.changeMinor > 0 && (
          <div className="flex justify-between font-bold"><span>الباقي</span><span>{formatMoney(sale.changeMinor)}</span></div>
        )}
        {settings.footer && (<><hr className="my-1 border-dashed border-black" /><div className="text-center">{settings.footer}</div></>)}
      </div>
    </>
  );
}
