import { Plus } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import Alert from "../../components/Alert";
import { btnPrimary, cardClass, inputClass } from "../../components/ui";
import { api, type Unit } from "../../services/api";

export default function UnitsTab() {
  const [units, setUnits] = useState<Unit[]>([]);
  const [form, setForm] = useState({ nameAr: "", nameEn: "", allowsDecimal: false });
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    api.listUnits().then(setUnits).catch((e) => setError(String(e)));
  }, []);
  useEffect(load, [load]);

  async function add(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    try {
      await api.createUnit(form);
      setForm({ nameAr: "", nameEn: "", allowsDecimal: false });
      load();
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="space-y-4">
      <form onSubmit={add} className={`${cardClass} flex flex-wrap items-end gap-3 p-4`}>
        <label className="text-sm">
          اسم الوحدة
          <input required className={`${inputClass} mt-1 w-48`} value={form.nameAr}
            onChange={(e) => setForm({ ...form, nameAr: e.target.value })} />
        </label>
        <label className="text-sm">
          بالإنجليزية
          <input dir="ltr" className={`${inputClass} mt-1 w-40`} value={form.nameEn}
            onChange={(e) => setForm({ ...form, nameEn: e.target.value })} />
        </label>
        <label className="flex items-center gap-2 pb-2 text-sm">
          <input type="checkbox" checked={form.allowsDecimal}
            onChange={(e) => setForm({ ...form, allowsDecimal: e.target.checked })} />
          تقبل كسورًا
        </label>
        <button className={btnPrimary} type="submit"><Plus size={16} /> إضافة وحدة</button>
      </form>
      {error && <Alert kind="error">{error}</Alert>}
      <div className={`${cardClass} overflow-hidden`}>
        <table className="w-full text-sm">
          <thead className="bg-slate-50 text-slate-500 dark:bg-slate-900/40">
            <tr>
              <th className="px-4 py-2 text-start">الوحدة</th>
              <th className="px-4 py-2 text-start">بالإنجليزية</th>
              <th className="px-4 py-2 text-start">كسور</th>
              <th className="px-4 py-2 text-start">النوع</th>
            </tr>
          </thead>
          <tbody>
            {units.map((u) => (
              <tr key={u.id} className="border-t border-slate-100 dark:border-slate-700">
                <td className="px-4 py-2 font-medium">{u.nameAr}</td>
                <td className="px-4 py-2" dir="ltr">{u.nameEn}</td>
                <td className="px-4 py-2">{u.allowsDecimal ? "نعم" : "لا"}</td>
                <td className="px-4 py-2 text-slate-500">{u.isSystem ? "أساسية" : "مضافة"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
