import { Printer } from "lucide-react";
import { useEffect, useState } from "react";
import { api, type SaleDetail } from "../services/api";
import { useSettingsStore } from "../store/settingsStore";
import Alert from "./Alert";
import Modal from "./Modal";
import Receipt, { receiptSettings } from "./Receipt";
import { btnGhost, btnPrimary } from "./ui";

/** معاينة الإيصال وطباعته عبر طباعة ويندوز (تعمل مع أي طابعة لها تعريف في ويندوز). */
export default function ReceiptDialog({ saleId, onClose }: { saleId: string; onClose: () => void }) {
  const settings = useSettingsStore((s) => s.settings);
  const [sale, setSale] = useState<SaleDetail | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.getSale(saleId).then(setSale).catch((e) => setError(String(e)));
  }, [saleId]);

  return (
    <Modal title="معاينة الإيصال" onClose={onClose}>
      {error && <Alert kind="error">{error}</Alert>}
      {sale && (
        <div className="space-y-4">
          <div className="max-h-[60vh] overflow-y-auto rounded-lg bg-slate-100 p-4 dark:bg-slate-900">
            <Receipt sale={sale} settings={receiptSettings(settings)} />
          </div>
          <div className="flex justify-end gap-2">
            <button className={btnGhost} onClick={onClose}>إغلاق</button>
            <button className={btnPrimary} onClick={() => window.print()}>
              <Printer size={16} /> طباعة
            </button>
          </div>
        </div>
      )}
    </Modal>
  );
}
