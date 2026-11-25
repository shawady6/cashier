import { Plus } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import Alert from "../../components/Alert";
import Modal from "../../components/Modal";
import PageHeader from "../../components/PageHeader";
import { btnGhost, btnPrimary, cardClass, inputClass } from "../../components/ui";
import { formatMoney } from "../../lib/format";
import { api, type Supplier, type SupplierInput } from "../../services/api";

const empty: SupplierInput = { nameAr: "", phone: "", address: "", notes: "", isActive: true };

export default function SuppliersPage() {
  const [rows, setRows] = useState<Supplier[]>([]);
  const [edit, setEdit] = useState<SupplierInput | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    api.listSuppliers().then(setRows).catch((e) => setError(String(e)));
  }, []);
  useEffect(load, [load]);

  async function save(e: React.FormEvent) {
    e.preventDefault();
    if (!edit) return;
    try {
      await api.saveSupplier(edit);
      setEdit(null);
      setError(null);
      load();
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="max-w-5xl">
      <PageHeader title="الموردون" description="بيانات الموردين والمبالغ المستحقة لهم (محسوبة من فواتير الشراء والمرتجعات والدفعات)." />
      {error && <Alert kind="error">{error}</Alert>}
      <div className="mb-3">
        <button className={btnPrimary} onClick={() => { setError(null); setEdit(empty); }}>
          <Plus size={16} /> مورد جديد
        </button>
      </div>
      <div className={`${cardClass} overflow-x-auto`}>
        <table className="w-full text-sm">
          <thead className="text-slate-500">
            <tr>
              <th className="p-3 text-start">الاسم</th>
              <th className="p-3 text-start">الهاتف</th>
              <th className="p-3 text-start">المستحق له</th>
              <th className="p-3 text-start">الحالة</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 && (
              <tr><td colSpan={5} className="p-6 text-center text-slate-400">لا يوجد موردون بعد.</td></tr>
            )}
            {rows.map((s) => (
              <tr key={s.id} className="border-t border-slate-100 dark:border-slate-700">
                <td className="p-3">{s.nameAr}</td>
                <td className="p-3" dir="ltr">{s.phone}</td>
                <td className="p-3">{formatMoney(s.balanceMinor)}</td>
                <td className="p-3">{s.isActive ? "نشط" : "موقوف"}</td>
                <td className="p-3 text-end">
                  <button className={btnGhost} onClick={() => { setError(null); setEdit({ id: s.id, nameAr: s.nameAr, phone: s.phone, address: s.address, notes: s.notes, isActive: s.isActive }); }}>
                    تعديل
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {edit && (
        <Modal title={edit.id ? "تعديل مورد" : "مورد جديد"} onClose={() => setEdit(null)}>
          <form onSubmit={save} className="space-y-3">
            {error && <Alert kind="error">{error}</Alert>}
            <label className="block text-sm">الاسم
              <input className={`${inputClass} mt-1`} value={edit.nameAr} onChange={(e) => setEdit({ ...edit, nameAr: e.target.value })} autoFocus />
            </label>
            <label className="block text-sm">الهاتف
              <input className={`${inputClass} mt-1`} dir="ltr" value={edit.phone} onChange={(e) => setEdit({ ...edit, phone: e.target.value })} />
            </label>
            <label className="block text-sm">العنوان
              <input className={`${inputClass} mt-1`} value={edit.address} onChange={(e) => setEdit({ ...edit, address: e.target.value })} />
            </label>
            <label className="block text-sm">ملاحظات
              <textarea className={`${inputClass} mt-1`} value={edit.notes} onChange={(e) => setEdit({ ...edit, notes: e.target.value })} />
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
