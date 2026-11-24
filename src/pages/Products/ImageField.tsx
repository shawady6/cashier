import { ImagePlus, Trash2 } from "lucide-react";
import { useRef, useState } from "react";
import { btnGhost } from "../../components/ui";

export const MAX_IMAGE_BYTES = 1_000_000;

export default function ImageField({
  preview,
  onPick,
  onRemove,
}: {
  preview: string | null;
  onPick: (dataUrl: string) => void;
  onRemove: () => void;
}) {
  const ref = useRef<HTMLInputElement>(null);
  const [error, setError] = useState<string | null>(null);

  function handle(file: File | undefined) {
    if (!file) return;
    setError(null);
    if (file.size > MAX_IMAGE_BYTES) {
      setError("حجم الصورة يجب ألا يتجاوز 1 ميجابايت");
      return;
    }
    const reader = new FileReader();
    reader.onload = () => onPick(String(reader.result));
    reader.readAsDataURL(file);
  }

  return (
    <div className="flex items-center gap-4">
      <div className="flex h-24 w-24 items-center justify-center overflow-hidden rounded-lg border border-dashed border-slate-300 dark:border-slate-600">
        {preview ? <img src={preview} alt="صورة المنتج" className="h-full w-full object-cover" /> : <ImagePlus className="text-slate-400" />}
      </div>
      <div className="space-y-2">
        <input ref={ref} type="file" accept="image/png,image/jpeg,image/webp" hidden onChange={(e) => handle(e.target.files?.[0])} />
        <button type="button" className={btnGhost} onClick={() => ref.current?.click()}>اختيار صورة</button>
        {preview && (
          <button type="button" className={`${btnGhost} text-red-600`} onClick={onRemove}>
            <Trash2 size={14} /> إزالة
          </button>
        )}
        <p className="text-xs text-slate-500">PNG أو JPG أو WEBP، حتى 1 ميجابايت</p>
        {error && <p role="alert" className="text-xs text-red-600">{error}</p>}
      </div>
    </div>
  );
}
