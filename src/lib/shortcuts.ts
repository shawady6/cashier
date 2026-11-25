export type ShortcutAction = "search" | "pay" | "suspend" | "resume" | "remove";

export const DEFAULT_SHORTCUTS: Record<ShortcutAction, string> = {
  search: "F2",
  pay: "F4",
  suspend: "F8",
  resume: "F9",
  remove: "Delete",
};

export const SHORTCUT_LABELS: Record<ShortcutAction, string> = {
  search: "البحث عن منتج",
  pay: "الدفع",
  suspend: "تعليق الفاتورة",
  resume: "استرجاع فاتورة معلّقة",
  remove: "حذف البند المحدد",
};

export function parseShortcuts(raw: string | undefined): Record<ShortcutAction, string> {
  try {
    return { ...DEFAULT_SHORTCUTS, ...(raw ? JSON.parse(raw) : {}) };
  } catch {
    return DEFAULT_SHORTCUTS;
  }
}
