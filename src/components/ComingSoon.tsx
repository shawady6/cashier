import PageHeader from "./PageHeader";

export default function ComingSoon({ title, phase }: { title: string; phase: number }) {
  return (
    <div>
      <PageHeader title={title} />
      <div className="rounded-xl border border-dashed border-slate-300 p-8 text-sm text-slate-600 dark:border-slate-600 dark:text-slate-300">
        هذه الصفحة غير متاحة بعد، وتُنفَّذ في المرحلة {phase} من خطة التطوير.
      </div>
    </div>
  );
}
