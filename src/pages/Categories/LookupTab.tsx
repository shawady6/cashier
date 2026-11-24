import { Pencil, Plus } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import Alert from "../../components/Alert";
import Modal from "../../components/Modal";
import { btnGhost, btnPrimary, cardClass, inputClass } from "../../components/ui";
import { api, type LookupItem, type LookupKind } from "../../services/api";

const labels: Record<LookupKind, { one: string; add: string }> = {
  category: { one: "التصنيف", add: "تصنيف جديد" },
  brand: { one: "الماركة", add: "ماركة جديدة" },
};

export default function LookupTab({ kind }: { kind: LookupKind }) {
  const [items, setItems] = useState<LookupItem[]>([]);
  const [editing, setEditing] = useState<Partial<LookupItem> | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const l = labels[kind];

  const load = useCallback(() => {
    api.listLookups(kind).then(setItems).catch((e) => setError(String(e)));
  }, [kind]);
  useEffect(load, [load]);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (!editing) return;
    setSaving(true);
    setError(null);
    try {
      await api.saveLookup(kind, {
        id: editing.id,
        nameAr: editing.nameAr ?? "",
        nameEn: editing.nameEn ?? "",
        isActive: editing.isActive ?? true,
      });
      setEditing(null);
      load();
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex justify-end">
        <button className={btnPrimary} onClick={() => { setError(null); setEditing({ isActive: true }); }}>
          <Plus size={16} /> {l.add}
        </button>
      </div>
      {error && !editing && <Alert kind="error">{error}</Alert>}
      <div className={`${cardClass} overflow-hidden`}>
        <table className="w-full text-sm">
          <thead className="bg-slate-50 text-slate-500 dark:bg-slate-900/40">
            <tr>
              <th className="px-4 py-2 text-start">الاسم</th>
              <th className="px-4 py-2 text-start">الاسم بالإنجليزية</th>
              <th className="px-4 py-2 text-start">عدد المنتجات</th>
              <th className="px-4 py-2 text-start">الحالة</th>
              <th className="w-12" />
            </tr>
          </thead>
          <tbody>
            {items.length === 0 && (
              <tr><td colSpan={5} className="px-4 py-8 text-center text-slate-500">لا توجد بيانات بعد</td></tr>
            )}
            {items.map((i) => (
              <tr key={i.id} className="border-t border-slate-100 dark:border-slate-700">
                <td className="px-4 py-2 font-medium">{i.nameAr}</td>
                <td className="px-4 py-2" dir="ltr">{i.nameEn}</td>
                <td className="px-4 py-2">{i.productCount.toLocaleString("ar-EG")}</td>
                <td className="px-4 py-2">
                  <span className={i.isActive ? "text-teal-700" : "text-slate-400"}>{i.isActive ? "نشط" : "غير نشط"}</span>
                </td>
                <td className="px-2">
                  <button aria-label="تعديل" className={btnGhost} onClick={() => { setError(null); setEditing(i); }}>
                    <Pencil size={14} />
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {editing && (
        <Modal title={editing.id ? `تعديل ${l.one}` : l.add} onClose={() => setEditing(null)}>
          <form onSubmit={submit} className="space-y-4">
            {error && <Alert kind="error">{error}</Alert>}
            <label className="block text-sm">
              الاسم بالعربية
              <input autoFocus required className={`${inputClass} mt-1`} value={editing.nameAr ?? ""}
                onChange={(e) => setEditing({ ...editing, nameAr: e.target.value })} />
            </label>
            <label className="block text-sm">
              الاسم بالإنجليزية (اختياري)
              <input dir="ltr" className={`${inputClass} mt-1`} value={editing.nameEn ?? ""}
                onChange={(e) => setEditing({ ...editing, nameEn: e.target.value })} />
            </label>
            <label className="flex items-center gap-2 text-sm">
              <input type="checkbox" checked={editing.isActive ?? true}
                onChange={(e) => setEditing({ ...editing, isActive: e.target.checked })} />
              نشط
            </label>
            <div className="flex justify-end gap-2">
              <button type="button" className={btnGhost} onClick={() => setEditing(null)}>إلغاء</button>
              <button type="submit" className={btnPrimary} disabled={saving}>حفظ</button>
            </div>
          </form>
        </Modal>
      )}
    </div>
  );
}
