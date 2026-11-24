export default function PageHeader({ title, description }: { title: string; description?: string }) {
  return (
    <div className="mb-6">
      <h2 className="text-xl font-bold">{title}</h2>
      {description && <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">{description}</p>}
    </div>
  );
}
