import { X } from "lucide-react";
import { useState } from "react";
import { inputClass } from "../../components/ui";

/** ماسح الباركود (لوحة مفاتيح USB) يكتب الأرقام ثم Enter: نضيف الباركود ولا نرسل النموذج. */
export default function BarcodeInput({
  value,
  onChange,
}: {
  value: string[];
  onChange: (v: string[]) => void;
}) {
  const [draft, setDraft] = useState("");

  function add() {
    const code = draft.trim();
    if (code && !value.includes(code)) onChange([...value, code]);
    setDraft("");
  }

  return (
    <div>
      <div className="flex flex-wrap gap-2">
        {value.map((b) => (
          <span key={b} dir="ltr" className="inline-flex items-center gap-1 rounded-full bg-slate-100 px-3 py-1 text-xs dark:bg-slate-700">
            {b}
            <button type="button" aria-label={`حذف الباركود ${b}`} onClick={() => onChange(value.filter((x) => x !== b))}>
              <X size={12} />
            </button>
          </span>
        ))}
      </div>
      <input
        dir="ltr"
        className={`${inputClass} mt-2`}
        placeholder="امسح الباركود أو اكتبه ثم اضغط Enter"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            add();
          }
        }}
        onBlur={add}
      />
    </div>
  );
}
