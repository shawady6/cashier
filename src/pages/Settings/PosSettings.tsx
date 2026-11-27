import { useEffect, useState } from "react";
import Alert from "../../components/Alert";
import { btnPrimary, inputClass } from "../../components/ui";
import { DEFAULT_SHORTCUTS, parseShortcuts, SHORTCUT_LABELS, type ShortcutAction } from "../../lib/shortcuts";
import { useSettingsStore } from "../../store/settingsStore";

// تحويل نص النسبة إلى نقاط أساسية (14.5 -> 1450) بدون float. Rust يتحقق من المدى.
function percentToBp(text: string): string {
  const m = /^(\d+)(?:[.,٫](\d{1,2}))?$/.exec(text.trim());
  if (!m) return "invalid";
  return String(Number(m[1]) * 100 + Number((m[2] ?? "").padEnd(2, "0") || 0));
}
function bpToPercent(bp: string | undefined): string {
  const n = Number(bp ?? "0");
  if (!Number.isInteger(n) || n < 0) return "0";
  const frac = String(n % 100).padStart(2, "0").replace(/0+$/, "");
  return frac ? `${Math.trunc(n / 100)}.${frac}` : String(Math.trunc(n / 100));
}

export default function PosSettings() {
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const [tax, setTax] = useState("0");
  const [footer, setFooter] = useState("");
  const [keys, setKeys] = useState<Record<ShortcutAction, string>>(DEFAULT_SHORTCUTS);
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null);

  useEffect(() => {
    setTax(bpToPercent(settings.tax_rate_bp));
    setFooter(settings.receipt_footer ?? "");
    setKeys(parseShortcuts(settings.pos_shortcuts));
  }, [settings.tax_rate_bp, settings.receipt_footer, settings.pos_shortcuts]);

  async function save(entries: [string, string][], okText: string) {
    try {
      for (const [k, v] of entries) await update(k, v);
      setMsg({ ok: true, text: okText });
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
    }
  }

  return (
    <>
      {msg && <Alert kind={msg.ok ? "ok" : "error"}>{msg.text}</Alert>}
      <div className="grid gap-4 md:grid-cols-2">
        <label className="block text-sm">
          نسبة الضريبة % (الأسعار شاملة الضريبة، 0 = بدون)
          <input inputMode="decimal" className={`${inputClass} mt-1`} value={tax} onChange={(e) => setTax(e.target.value)} />
        </label>
        <label className="block text-sm">
          عرض ورق الطابعة
          <select
            className={`${inputClass} mt-1`}
            value={settings.receipt_paper_width ?? "80"}
            onChange={(e) => void save([["receipt_paper_width", e.target.value]], "تم حفظ عرض الورق")}
          >
            <option value="80">80 مم</option>
            <option value="58">58 مم</option>
          </select>
        </label>
      </div>
      <label className="block text-sm">
        نص أسفل الإيصال
        <input className={`${inputClass} mt-1`} value={footer} onChange={(e) => setFooter(e.target.value)} />
      </label>
      <div>
        <div className="mb-2 text-sm font-medium">اختصارات الكاشير</div>
        <div className="grid gap-3 md:grid-cols-3">
          {(Object.keys(SHORTCUT_LABELS) as ShortcutAction[]).map((a) => (
            <label key={a} className="block text-sm">
              {SHORTCUT_LABELS[a]}
              <select className={`${inputClass} mt-1`} value={keys[a]} onChange={(e) => setKeys({ ...keys, [a]: e.target.value })}>
                {[...Array.from({ length: 9 }, (_, i) => `F${i + 1}`), "F10", "Delete"].map((k) => <option key={k}>{k}</option>)}
              </select>
            </label>
          ))}
        </div>
        <p className="mt-1 text-xs text-slate-500">لا يمكن تكرار نفس المفتاح. Escape يغلق النوافذ دائمًا.</p>
      </div>
      <button
        className={btnPrimary}
        onClick={() => {
          const bp = percentToBp(tax);
          if (bp === "invalid") return setMsg({ ok: false, text: "نسبة الضريبة غير صالحة" });
          void save(
            [["tax_rate_bp", bp], ["receipt_footer", footer.trim()], ["pos_shortcuts", JSON.stringify(keys)]],
            "تم حفظ إعدادات الكاشير",
          );
        }}
      >
        حفظ إعدادات الكاشير
      </button>
    </>
  );
}
