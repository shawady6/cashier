import { useState } from "react";
import PageHeader from "../../components/PageHeader";
import LookupTab from "./LookupTab";
import UnitsTab from "./UnitsTab";

const tabs = [
  { id: "category", label: "التصنيفات" },
  { id: "brand", label: "الماركات" },
  { id: "units", label: "الوحدات" },
] as const;

export default function CategoriesPage() {
  const [tab, setTab] = useState<(typeof tabs)[number]["id"]>("category");
  return (
    <div className="max-w-4xl">
      <PageHeader title="التصنيفات والماركات والوحدات" description="بيانات أساسية تُستخدم عند تعريف المنتجات." />
      <div role="tablist" className="mb-4 flex gap-1 border-b border-slate-200 dark:border-slate-700">
        {tabs.map((t) => (
          <button
            key={t.id}
            role="tab"
            aria-selected={tab === t.id}
            onClick={() => setTab(t.id)}
            className={`px-4 py-2 text-sm ${
              tab === t.id ? "border-b-2 border-teal-600 font-semibold text-teal-700" : "text-slate-500"
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>
      {tab === "units" ? <UnitsTab /> : <LookupTab key={tab} kind={tab} />}
    </div>
  );
}
