import { useState } from "react";
import Modal from "../../components/Modal";
import { btnGhost, btnPrimary, inputClass } from "../../components/ui";
import type { DiscountInput } from "../../services/api";

export default function DiscountDialog({
  title,
  initial,
  onSave,
  onClose,
}: {
  title: string;
  initial: DiscountInput | null;
  onSave: (d: DiscountInput | null) => void;
  onClose: () => void;
}) {
  const [kind, setKind] = useState<DiscountInput["kind"]>(initial?.kind ?? "percent");
  const [value, setValue] = useState(initial?.value ?? "");

  function submit(e: React.FormEvent) {
    e.preventDefault();
    onSave(value.trim() ? { kind, value: value.trim() } : null);
  }

  return (
    <Modal title={title} onClose={onClose}>
      <form onSubmit={submit} className="space-y-4">
        <div className="flex gap-4 text-sm">
          <label className="flex items-center gap-2">
            <input type="radio" checked={kind === "percent"} onChange={() => setKind("percent")} /> نسبة %
          </label>
          <label className="flex items-center gap-2">
            <input type="radio" checked={kind === "amount"} onChange={() => setKind("amount")} /> مبلغ (جنيه)
          </label>
        </div>
        <input autoFocus inputMode="decimal" className={inputClass} value={value} onChange={(e) => setValue(e.target.value)} placeholder="اتركه فارغًا لإلغاء الخصم" />
        <div className="flex justify-end gap-2">
          <button type="button" className={btnGhost} onClick={onClose}>إلغاء</button>
          <button type="submit" className={btnPrimary}>تطبيق</button>
        </div>
      </form>
    </Modal>
  );
}
