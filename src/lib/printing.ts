import type { ReceiptSettings } from "../components/Receipt";
import { api, type SaleDetail } from "../services/api";
import { renderReceipt } from "./receiptRaster";

export type DrawerMode = "off" | "cash" | "always";

export function drawerMode(s: Record<string, string>): DrawerMode {
  const v = s.drawer_mode;
  return v === "cash" || v === "always" ? v : "off";
}

export function isThermal(s: Record<string, string>): boolean {
  return s.printer_mode === "thermal" && !!s.printer_name;
}

export function shouldOpenDrawer(s: Record<string, string>, methods: string[]): boolean {
  const mode = drawerMode(s);
  return mode === "always" || (mode === "cash" && methods.includes("cash"));
}

/** طباعة إيصال بيع على الطابعة الحرارية المختارة، مع فتح الدُرج عند الطلب. */
export async function printSaleThermal(sale: SaleDetail, receipt: ReceiptSettings, openDrawer: boolean): Promise<void> {
  await api.printReceiptImage(renderReceipt(sale, receipt), openDrawer);
}
