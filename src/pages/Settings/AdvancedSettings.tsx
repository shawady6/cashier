import { useCallback, useEffect, useState } from "react";
import Alert from "../../components/Alert";
import { receiptSettings } from "../../components/Receipt";
import { btnGhost, btnPrimary, inputClass } from "../../components/ui";
import { renderTestPage } from "../../lib/receiptRaster";
import { scannerConfig } from "../../lib/scanner";
import { api, type PrinterInfo } from "../../services/api";
import { useSettingsStore } from "../../store/settingsStore";

type Msg = { ok: boolean; text: string } | null;

function useSave() {
  const update = useSettingsStore((s) => s.update);
  const [msg, setMsg] = useState<Msg>(null);
  const save = useCallback(
    async (entries: [string, string][], okText?: string) => {
      try {
        for (const [k, v] of entries) await update(k, v);
        setMsg(okText ? { ok: true, text: okText } : null);
      } catch (e) {
        setMsg({ ok: false, text: String(e) });
      }
    },
    [update],
  );
  return { msg, setMsg, save };
}

function Group({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="space-y-3 rounded-lg border border-slate-200 p-4 dark:border-slate-700">
      <div className="font-semibold">{title}</div>
      {children}
    </div>
  );
}

function PrinterPanel() {
  const settings = useSettingsStore((s) => s.settings);
  const { msg, setMsg, save } = useSave();
  const [printers, setPrinters] = useState<PrinterInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const selected = settings.printer_name ?? "";

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      setPrinters(await api.listPrinters());
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
    } finally {
      setLoading(false);
    }
  }, [setMsg]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const missing = selected !== "" && !printers.some((p) => p.name === selected);

  async function test() {
    setBusy(true);
    setMsg(null);
    try {
      await api.printReceiptImage(renderTestPage(receiptSettings(settings)), false);
      setMsg({ ok: true, text: "تم إرسال الصفحة التجريبية للطابعة" });
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
    } finally {
      setBusy(false);
    }
  }

  return (
    <Group title="طابعة الإيصالات">
      {msg && <Alert kind={msg.ok ? "ok" : "error"}>{msg.text}</Alert>}
      <div className="grid gap-3 sm:grid-cols-2">
        <label className="block text-sm">
          الطابعة
          <select className={`${inputClass} mt-1`} value={selected} onChange={(e) => void save([["printer_name", e.target.value]], "تم حفظ الطابعة")}>
            <option value="">— لم تُختر —</option>
            {missing && <option value={selected}>{selected} (غير متصلة)</option>}
            {printers.map((p) => (
              <option key={p.name} value={p.name}>
                {p.name}
                {p.isDefault ? " (الافتراضية)" : ""}
              </option>
            ))}
          </select>
        </label>
        <label className="block text-sm">
          طريقة الطباعة
          <select
            className={`${inputClass} mt-1`}
            value={settings.printer_mode === "thermal" ? "thermal" : "browser"}
            onChange={(e) => void save([["printer_mode", e.target.value]], "تم حفظ طريقة الطباعة")}
          >
            <option value="browser">نافذة طباعة ويندوز</option>
            <option value="thermal">حرارية مباشرة (ESC/POS)</option>
          </select>
        </label>
        <label className="block text-sm">
          عرض الورق
          <select
            className={`${inputClass} mt-1`}
            value={settings.receipt_paper_width ?? "80"}
            onChange={(e) => void save([["receipt_paper_width", e.target.value]], "تم حفظ عرض الورق")}
          >
            <option value="80">80 مم</option>
            <option value="58">58 مم</option>
          </select>
        </label>
        <label className="block text-sm">
          عدد النسخ
          <select className={`${inputClass} mt-1`} value={settings.print_copies ?? "1"} onChange={(e) => void save([["print_copies", e.target.value]], "تم حفظ عدد النسخ")}>
            {["1", "2", "3"].map((n) => (
              <option key={n}>{n}</option>
            ))}
          </select>
        </label>
      </div>
      <div className="flex flex-wrap gap-x-6 gap-y-2 text-sm">
        <label className="flex items-center gap-2">
          <input type="checkbox" checked={settings.print_auto === "1"} onChange={(e) => void save([["print_auto", e.target.checked ? "1" : "0"]], "تم الحفظ")} />
          طباعة تلقائية بعد البيع
        </label>
        <label className="flex items-center gap-2">
          <input type="checkbox" checked={settings.printer_cut !== "0"} onChange={(e) => void save([["printer_cut", e.target.checked ? "1" : "0"]], "تم الحفظ")} />
          قص الورق بعد الطباعة
        </label>
      </div>
      <div className="flex gap-2">
        <button type="button" className={btnPrimary} disabled={busy || !selected} onClick={() => void test()}>
          طباعة تجريبية
        </button>
        <button type="button" className={btnGhost} disabled={loading} onClick={() => void refresh()}>
          تحديث القائمة
        </button>
      </div>
    </Group>
  );
}

function DrawerPanel() {
  const settings = useSettingsStore((s) => s.settings);
  const { msg, setMsg, save } = useSave();
  const [busy, setBusy] = useState(false);

  async function test() {
    setBusy(true);
    setMsg(null);
    try {
      await api.openCashDrawer();
      setMsg({ ok: true, text: "تم إرسال أمر فتح الدُرج" });
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
    } finally {
      setBusy(false);
    }
  }

  return (
    <Group title="دُرج الكاش">
      {msg && <Alert kind={msg.ok ? "ok" : "error"}>{msg.text}</Alert>}
      <div className="grid gap-3 sm:grid-cols-2">
        <label className="block text-sm">
          فتح الدُرج
          <select className={`${inputClass} mt-1`} value={settings.drawer_mode ?? "off"} onChange={(e) => void save([["drawer_mode", e.target.value]], "تم حفظ الإعداد")}>
            <option value="off">لا يفتح تلقائيًا</option>
            <option value="cash">عند الدفع النقدي</option>
            <option value="always">عند أي بيع</option>
          </select>
        </label>
        <label className="block text-sm">
          منفذ الدُرج
          <select className={`${inputClass} mt-1`} value={settings.drawer_pin === "1" ? "1" : "0"} onChange={(e) => void save([["drawer_pin", e.target.value]], "تم حفظ المنفذ")}>
            <option value="0">الأساسي</option>
            <option value="1">البديل</option>
          </select>
        </label>
      </div>
      <button type="button" className={btnPrimary} disabled={busy || !settings.printer_name} onClick={() => void test()}>
        فتح الدُرج الآن
      </button>
    </Group>
  );
}

interface ScanSample {
  code: string;
  length: number;
  avgGap: number;
  maxGap: number;
}

function ScannerPanel() {
  const settings = useSettingsStore((s) => s.settings);
  const { msg, save } = useSave();
  const cfg = scannerConfig(settings);
  const [minLen, setMinLen] = useState(String(cfg.minLength));
  const [gap, setGap] = useState(String(cfg.maxGapMs));
  const [typed, setTyped] = useState<{ t: number; ch: string }[]>([]);
  const [sample, setSample] = useState<ScanSample | null>(null);

  useEffect(() => {
    setMinLen(String(cfg.minLength));
    setGap(String(cfg.maxGapMs));
  }, [cfg.minLength, cfg.maxGapMs]);

  function onKey(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") {
      e.preventDefault();
      if (typed.length > 0) {
        const gaps = typed.slice(1).map((k, i) => k.t - typed[i].t);
        setSample({
          code: typed.map((k) => k.ch).join(""),
          length: typed.length,
          avgGap: gaps.length ? Math.round(gaps.reduce((a, b) => a + b, 0) / gaps.length) : 0,
          maxGap: gaps.length ? Math.round(Math.max(...gaps)) : 0,
        });
      }
      setTyped([]);
      e.currentTarget.value = "";
    } else if (e.key.length === 1) {
      setTyped((t) => [...t, { t: performance.now(), ch: e.key }]);
    }
  }

  const verdict = !sample
    ? null
    : sample.length < cfg.minLength
      ? { ok: false, text: "الكود أقصر من أقل طول محدد" }
      : sample.maxGap > cfg.maxGapMs
        ? { ok: false, text: "الإدخال أبطأ من الفاصل المحدد، يُعتبر كتابة يدوية" }
        : { ok: true, text: "قراءة سكانر سليمة" };

  async function saveNumbers() {
    await save(
      [
        ["scanner_min_length", minLen.trim()],
        ["scanner_max_gap_ms", gap.trim()],
      ],
      "تم حفظ إعدادات السكانر",
    );
  }

  return (
    <Group title="الباركود سكانر">
      {msg && <Alert kind={msg.ok ? "ok" : "error"}>{msg.text}</Alert>}
      <label className="block text-sm">
        اختبار القراءة (امسح أي باركود هنا)
        <input className={`${inputClass} mt-1`} dir="ltr" onKeyDown={onKey} placeholder="…" />
      </label>
      {sample && verdict && (
        <div className="space-y-2 rounded-lg bg-slate-50 p-3 text-sm dark:bg-slate-900">
          <div dir="ltr" className="font-mono text-base">{sample.code}</div>
          <div>الطول: {sample.length} · متوسط الفاصل: {sample.avgGap} مللي ث · أبطأ فاصل: {sample.maxGap} مللي ث</div>
          <div className={verdict.ok ? "font-semibold text-teal-700" : "font-semibold text-amber-700"}>{verdict.text}</div>
          {sample.maxGap > cfg.maxGapMs && (
            <button
              type="button"
              className={btnGhost}
              onClick={() => void save([["scanner_max_gap_ms", String(Math.min(500, Math.max(10, sample.maxGap * 2)))]], "تم ضبط الفاصل من آخر قراءة")}
            >
              ضبط الفاصل من هذه القراءة
            </button>
          )}
        </div>
      )}
      <div className="grid gap-3 sm:grid-cols-2">
        <label className="block text-sm">
          أقل طول للباركود
          <input className={`${inputClass} mt-1`} dir="ltr" inputMode="numeric" value={minLen} onChange={(e) => setMinLen(e.target.value)} />
        </label>
        <label className="block text-sm">
          أقصى فاصل بين الحروف (مللي ثانية)
          <input className={`${inputClass} mt-1`} dir="ltr" inputMode="numeric" value={gap} onChange={(e) => setGap(e.target.value)} />
        </label>
      </div>
      <label className="flex items-center gap-2 text-sm">
        <input type="checkbox" checked={cfg.global} onChange={(e) => void save([["scanner_global", e.target.checked ? "1" : "0"]], "تم الحفظ")} />
        المسح يعمل في شاشة الكاشير حتى لو المؤشر خارج خانة البحث
      </label>
      <button type="button" className={btnGhost} onClick={() => void saveNumbers()}>
        حفظ إعدادات السكانر
      </button>
    </Group>
  );
}

export default function AdvancedSettings() {
  return (
    <div className="space-y-4">
      <PrinterPanel />
      <DrawerPanel />
      <ScannerPanel />
    </div>
  );
}
