import type { ReactNode } from "react";

type Props = {
  id: string;
  title: string;
  open: boolean;
  onToggle: (id: string) => void;
  children: ReactNode;
};

export default function AccordionSection({ id, title, open, onToggle, children }: Props) {
  return (
    <section className="overflow-hidden rounded-xl border border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-800">
      <h3>
        <button
          type="button"
          id={`acc-${id}-btn`}
          aria-expanded={open}
          aria-controls={`acc-${id}-panel`}
          onClick={() => onToggle(id)}
          className="flex w-full items-center justify-between gap-3 px-5 py-4 text-start font-semibold hover:bg-slate-50 dark:hover:bg-slate-700/40"
        >
          <span>{title}</span>
          <svg
            aria-hidden="true"
            viewBox="0 0 20 20"
            className={`h-4 w-4 shrink-0 text-slate-500 transition-transform ${open ? "rotate-180" : ""}`}
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
          >
            <path d="M5 8l5 5 5-5" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </button>
      </h3>
      {open && (
        <div
          id={`acc-${id}-panel`}
          role="region"
          aria-labelledby={`acc-${id}-btn`}
          className="space-y-4 border-t border-slate-200 p-5 dark:border-slate-700"
        >
          {children}
        </div>
      )}
    </section>
  );
}
