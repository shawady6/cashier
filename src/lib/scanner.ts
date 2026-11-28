import { useEffect, useRef } from "react";

export interface ScannerConfig {
  minLength: number;
  maxGapMs: number;
  global: boolean;
}

export function scannerConfig(s: Record<string, string>): ScannerConfig {
  const min = Number(s.scanner_min_length);
  const gap = Number(s.scanner_max_gap_ms);
  return {
    minLength: Number.isInteger(min) && min >= 1 && min <= 30 ? min : 4,
    maxGapMs: Number.isInteger(gap) && gap >= 10 && gap <= 500 ? gap : 50,
    global: s.scanner_global !== "0",
  };
}

export interface ScanEvent {
  code: string;
  avgGapMs: number;
}

/** يجمّع ضغطات المفاتيح السريعة المنتهية بـ Enter كقراءة سكانر. الكتابة اليدوية البطيئة لا تُعتبر مسحًا. */
export function createScanDetector(cfg: ScannerConfig, now: () => number = () => performance.now()) {
  let buf = "";
  let last = 0;
  let gaps: number[] = [];
  const reset = () => {
    buf = "";
    gaps = [];
  };
  return {
    key(key: string): ScanEvent | null {
      const t = now();
      if (key === "Enter") {
        const ok = buf.length >= cfg.minLength;
        const avg = gaps.length ? gaps.reduce((a, b) => a + b, 0) / gaps.length : 0;
        const code = buf;
        reset();
        return ok ? { code, avgGapMs: Math.round(avg) } : null;
      }
      if (key.length !== 1) return null;
      if (buf && t - last > cfg.maxGapMs) reset();
      if (buf) gaps.push(t - last);
      buf += key;
      last = t;
      return null;
    },
  };
}

function isEditable(el: EventTarget | null): boolean {
  const t = el as HTMLElement | null;
  return !!t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.tagName === "SELECT" || t.isContentEditable);
}

/** مسح الباركود يعمل حتى لو المؤشر ليس في خانة البحث. */
export function useGlobalScan(cfg: ScannerConfig, enabled: boolean, onCode: (code: string) => void) {
  const cb = useRef(onCode);
  cb.current = onCode;
  useEffect(() => {
    if (!enabled || !cfg.global) return;
    const det = createScanDetector(cfg);
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.altKey || e.metaKey) return;
      const ev = det.key(e.key);
      if (ev && !isEditable(e.target)) {
        e.preventDefault();
        cb.current(ev.code);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [cfg.minLength, cfg.maxGapMs, cfg.global, enabled]);
}
