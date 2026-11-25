import { Plus } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import Alert from "../../components/Alert";
import Modal from "../../components/Modal";
import { btnGhost, btnPrimary, cardClass, inputClass } from "../../components/ui";
import { formatMoney } from "../../lib/format";
import { api, type Warehouse } from "../../services/api";

interface Edit { id?: string; nameAr: string; isActive: boolean; isDefault: boolean }

export default function WarehousesTab() {
  const [rows, setRows] = useState<Warehouse[]>([]);
  const [edit, setEdit] = useState<Edit | null>(null);
  const [error, setError] = useState<string | null>(null);
  const load = useCallback(() => { api.listWarehouses().then(setRows).catch((e) => setError(String(e))); }, []);
  useEffect(load, [load]);

  async function save(e: React.FormEvent) {
    e.preventDefault();
    if (!edit) return;
    try {
      await api.saveWarehouse(edit);
      setEdit(null);
      setError(null);
      load();
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="space-y-3">
      {error && <Alert kind="error">{error}</Alert>}
      <button className={btnPrimary} onClick={() => { setError(null); setEdit({ nameAr: "", isActive: true, isDefault: false }); }}>
        <Plus size={16} /> مخزن جديد
      </button>
      <div className={`${cardClass} overflow-x-auto`}>
        <table className="w-full text-sm">
          <thead className="text-slate-500">
            <tr><th className="p-3 text-start">المخزن</th><th className="p-3 text-start">قيمة المخزون</th><th className="p-3 text-start">الحالة</th><th /></tr>
          </thead>
          <tbody>
            {rows.map((w) => (
              <tr key={w.id} className="border-t border-slate-100 dark:border-slate-700">
                <td className="p-3">{w.nameAr}{w.isDefault ? " (افتراضي)" : ""}</td>
                <td className="p-3">{formatMoney(w.stockValueMinor)}</td>
                <td className="p-3">{w.isActive ? "نشط" : "موقوف"}</td>
                <td className="p-3 text-end">
                  <button className={btnGhost} onClick={() => { setError(null); setEdit({ id: w.id, nameAr: w.nameAr, isActive: w.isActive, isDefault: w.isDefault }); }}>تعديل</button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {edit && (
        <Modal title={edit.id ? "تعديل مخزن" : "مخزن جديد"} onClose={() => setEdit(null)}>
          <form onSubmit={save} className="space-y-3">
            {error && <Alert kind="error">{error}</Alert>}
            <label className="block text-sm">الاسم
              <input className={`${inputClass} mt-1`} value={edit.nameAr} onChange={(e) => setEdit({ ...edit, nameAr: e.target.value })} autoFocus />
            </label>
            <label className="flex items-center gap-2 text-sm">
              <input type="checkbox" checked={edit.isDefault} onChange={(e) => setEdit({ ...edit, isDefault: e.target.checked })} /> المخزن الافتراضي
            </label>
            <label className="flex items-center gap-2 text-sm">
              <input type="checkbox" checked={edit.isActive} onChange={(e) => setEdit({ ...edit, isActive: e.target.checked })} /> نشط
            </label>
            <div className="flex justify-end gap-2">
              <button type="button" className={btnGhost} onClick={() => setEdit(null)}>إلغاء</button>
              <button className={btnPrimary}>حفظ</button>
            </div>
          </form>
        </Modal>
      )}
    </div>
  );
}
