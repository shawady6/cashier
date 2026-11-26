import { useEffect, useState } from "react";
import { api, type AppInfo, type DbHealth } from "../../services/api";

/** معلومات تقنية للدعم والصيانة. */
export default function AboutSection() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [health, setHealth] = useState<DbHealth | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([api.getAppInfo(), api.getDbHealth()])
      .then(([i, h]) => {
        setInfo(i);
        setHealth(h);
      })
      .catch((e) => setError(String(e)));
  }, []);

  const rows: [string, string][] = [
    ["إصدار البرنامج", info?.appVersion ?? "…"],
    ["إصدار SQLite", health?.sqliteVersion ?? "…"],
    ["وضع السجل (Journal)", health?.journalMode ?? "…"],
    ["المفاتيح الأجنبية", health ? (health.foreignKeys ? "مفعّلة" : "غير مفعّلة") : "…"],
    ["تحديثات قاعدة البيانات المطبّقة", String(health?.appliedMigrations ?? "…")],
    ["مجلد البيانات", info?.dataDir ?? "…"],
  ];

  return (
    <section className="space-y-3 rounded-xl border border-slate-200 bg-white p-5 dark:border-slate-700 dark:bg-slate-800">
      <h3 className="font-semibold">حول البرنامج</h3>
      {error && <div role="alert" className="rounded-lg bg-red-50 p-3 text-sm text-red-700">تعذّر قراءة حالة النظام: {error}</div>}
      <dl className="grid gap-2 text-sm sm:grid-cols-2">
        {rows.map(([k, v]) => (
          <div key={k}>
            <dt className="text-xs text-slate-500 dark:text-slate-400">{k}</dt>
            <dd className="break-all font-medium" dir="auto">{v}</dd>
          </div>
        ))}
      </dl>
    </section>
  );
}
