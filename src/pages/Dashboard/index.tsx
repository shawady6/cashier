import { useEffect, useState } from "react";
import PageHeader from "../../components/PageHeader";
import StatCard from "../../components/StatCard";
import { api, type AppInfo, type DbHealth } from "../../services/api";

export default function Dashboard() {
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

  return (
    <div>
      <PageHeader
        title="حالة النظام"
        description="بيانات المبيعات والمخزون تظهر هنا بعد تفعيل الكاشير والمنتجات في المراحل القادمة."
      />
      {error && (
        <div role="alert" className="mb-4 rounded-lg bg-red-50 p-3 text-sm text-red-700">
          تعذّر قراءة حالة النظام: {error}
        </div>
      )}
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
        <StatCard label="إصدار البرنامج" value={info?.appVersion ?? "…"} />
        <StatCard label="إصدار SQLite" value={health?.sqliteVersion ?? "…"} />
        <StatCard label="وضع السجل (Journal)" value={health?.journalMode ?? "…"} />
        <StatCard
          label="المفاتيح الأجنبية"
          value={health ? (health.foreignKeys ? "مفعّلة" : "غير مفعّلة") : "…"}
        />
        <StatCard label="تحديثات قاعدة البيانات المطبّقة" value={String(health?.appliedMigrations ?? "…")} />
        <StatCard label="مجلد البيانات" value={info?.dataDir ?? "…"} />
      </div>
    </div>
  );
}
