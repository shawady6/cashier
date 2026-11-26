import { useEffect, useState } from "react";
import AccordionSection from "../../components/AccordionSection";
import PageHeader from "../../components/PageHeader";
import AboutSection from "./AboutSection";
import PosSettings from "./PosSettings";
import { api, type BusinessProfile } from "../../services/api";
import { useSettingsStore } from "../../store/settingsStore";

const inputClass =
  "w-full rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm dark:border-slate-600 dark:bg-slate-900";

export default function SettingsPage() {
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const [profiles, setProfiles] = useState<BusinessProfile[]>([]);
  const [form, setForm] = useState({ store_name: "", store_address: "", store_phone: "" });
  const [openId, setOpenId] = useState<string | null>(() => {
    try {
      return sessionStorage.getItem("settings.open");
    } catch {
      return null;
    }
  });
  const toggle = (id: string) => {
    const next = openId === id ? null : id;
    setOpenId(next);
    try {
      if (next) sessionStorage.setItem("settings.open", next);
      else sessionStorage.removeItem("settings.open");
    } catch {
      /* التخزين غير متاح */
    }
  };
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null);

  useEffect(() => {
    api
      .listBusinessProfiles()
      .then(setProfiles)
      .catch((e) => setMsg({ ok: false, text: String(e) }));
  }, []);

  useEffect(() => {
    setForm({
      store_name: settings.store_name ?? "",
      store_address: settings.store_address ?? "",
      store_phone: settings.store_phone ?? "",
    });
  }, [settings.store_name, settings.store_address, settings.store_phone]);

  async function save(entries: [string, string][], okText: string) {
    try {
      for (const [key, value] of entries) await update(key, value);
      setMsg({ ok: true, text: okText });
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
    }
  }

  return (
    <div className="max-w-2xl space-y-3">
      <PageHeader title="الإعدادات" />
      {msg && (
        <div
          role="status"
          className={`rounded-lg p-3 text-sm ${msg.ok ? "bg-teal-50 text-teal-800" : "bg-red-50 text-red-700"}`}
        >
          {msg.text}
        </div>
      )}

      <AccordionSection id="store" title="بيانات المتجر" open={openId === "store"} onToggle={toggle}>
        <label className="block text-sm">
          اسم المتجر
          <input
            className={`${inputClass} mt-1`}
            value={form.store_name}
            onChange={(e) => setForm({ ...form, store_name: e.target.value })}
          />
        </label>
        <label className="block text-sm">
          العنوان
          <input
            className={`${inputClass} mt-1`}
            value={form.store_address}
            onChange={(e) => setForm({ ...form, store_address: e.target.value })}
          />
        </label>
        <label className="block text-sm">
          الهاتف
          <input
            className={`${inputClass} mt-1`}
            dir="ltr"
            value={form.store_phone}
            onChange={(e) => setForm({ ...form, store_phone: e.target.value })}
          />
        </label>
        <button
          type="button"
          className="rounded-lg bg-teal-600 px-4 py-2 text-sm font-semibold text-white hover:bg-teal-700"
          onClick={() =>
            void save(
              [
                ["store_name", form.store_name.trim()],
                ["store_address", form.store_address.trim()],
                ["store_phone", form.store_phone.trim()],
              ],
              "تم حفظ بيانات المتجر",
            )
          }
        >
          حفظ بيانات المتجر
        </button>
      </AccordionSection>

      <AccordionSection id="business" title="نوع النشاط" open={openId === "business"} onToggle={toggle}>
        <select
          className={inputClass}
          value={settings.business_profile_code ?? ""}
          onChange={(e) => void save([["business_profile_code", e.target.value]], "تم تغيير نوع النشاط")}
        >
          {profiles.map((p) => (
            <option key={p.code} value={p.code}>
              {p.nameAr}
            </option>
          ))}
        </select>
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={settings.allow_negative_stock === "1"}
            onChange={(e) =>
              void save(
                [["allow_negative_stock", e.target.checked ? "1" : "0"]],
                "تم تحديث إعداد المخزون السالب",
              )
            }
          />
          السماح بالبيع عند نفاد المخزون (مخزون سالب)
        </label>
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={settings.require_cash_session === "1"}
            onChange={(e) =>
              void save([["require_cash_session", e.target.checked ? "1" : "0"]], "تم تحديث إعداد وردية الخزينة")
            }
          />
          إلزام فتح وردية خزينة قبل أي بيع أو حركة مالية
        </label>
      </AccordionSection>

      <AccordionSection id="pos" title="الكاشير والإيصال" open={openId === "pos"} onToggle={toggle}>
        <PosSettings />
      </AccordionSection>
      <AccordionSection id="about" title="حول البرنامج" open={openId === "about"} onToggle={toggle}>
        <AboutSection />
      </AccordionSection>
    </div>
  );
}
