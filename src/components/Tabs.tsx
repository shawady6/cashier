export default function Tabs<T extends string>({
  tabs,
  value,
  onChange,
}: {
  tabs: readonly { id: T; label: string }[];
  value: T;
  onChange: (id: T) => void;
}) {
  return (
    <div role="tablist" className="mb-4 flex flex-wrap gap-1 border-b border-slate-200 dark:border-slate-700">
      {tabs.map((t) => (
        <button
          key={t.id}
          role="tab"
          aria-selected={value === t.id}
          onClick={() => onChange(t.id)}
          className={`px-4 py-2 text-sm ${
            value === t.id ? "border-b-2 border-teal-600 font-semibold text-teal-700" : "text-slate-500"
          }`}
        >
          {t.label}
        </button>
      ))}
    </div>
  );
}
