import { Play, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import Alert from "../../components/Alert";
import Modal from "../../components/Modal";
import { btnGhost, btnPrimary } from "../../components/ui";
import { api, type SuspendedSale } from "../../services/api";

export default function SuspendedDialog({ onResume, onClose }: { onResume: (s: SuspendedSale) => void; onClose: () => void }) {
  const [items, setItems] = useState<SuspendedSale[]>([]);
  const [error, setError] = useState<string | null>(null);

  const load = () => api.listSuspendedSales().then(setItems).catch((e) => setError(String(e)));
  useEffect(() => { void load(); }, []);

  async function resume(id: string) {
    try {
      onResume(await api.resumeSale(id));
    } catch (e) {
      setError(String(e));
    }
  }

  async function discard(id: string) {
    if (!window.confirm("حذف الفاتورة المعلّقة نهائيًا؟")) return;
    try {
      await api.discardSuspendedSale(id);
      await load();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <Modal title="الفواتير المعلّقة" onClose={onClose}>
      {error && <Alert kind="error">{error}</Alert>}
      {items.length === 0 && <p className="py-6 text-center text-sm text-slate-500">لا توجد فواتير معلّقة</p>}
      <ul className="space-y-2">
        {items.map((s) => {
          let count = 0;
          try { count = (JSON.parse(s.payload).lines ?? []).length; } catch { /* payload تالف */ }
          return (
            <li key={s.id} className="flex items-center justify-between rounded-lg border border-slate-200 p-3 dark:border-slate-700">
              <div>
                <div className="font-medium">{s.label || "فاتورة معلّقة"}</div>
                <div className="text-xs text-slate-500">{new Date(s.createdAt).toLocaleString("ar-EG")} · {count} بند</div>
              </div>
              <div className="flex gap-2">
                <button className={btnPrimary} onClick={() => resume(s.id)}><Play size={14} /> استرجاع</button>
                <button className={`${btnGhost} text-red-600`} aria-label="حذف" onClick={() => discard(s.id)}><Trash2 size={14} /></button>
              </div>
            </li>
          );
        })}
      </ul>
    </Modal>
  );
}
