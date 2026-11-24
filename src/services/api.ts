import { invoke } from "@tauri-apps/api/core";

export interface AppInfo {
  appVersion: string;
  dataDir: string;
}

export interface DbHealth {
  sqliteVersion: string;
  foreignKeys: boolean;
  journalMode: string;
  appliedMigrations: number;
}

export interface BusinessProfile {
  id: string;
  code: string;
  nameAr: string;
  nameEn: string;
  trackExpiry: boolean;
  trackBatches: boolean;
  allowWeightedProducts: boolean;
  allowVariants: boolean;
}

/** كل الحسابات والتحقق في Rust. هذه الطبقة مجرد جسر للأوامر. */
export const api = {
  getAppInfo: () => invoke<AppInfo>("get_app_info"),
  getDbHealth: () => invoke<DbHealth>("get_db_health"),
  getSettings: () => invoke<Record<string, string>>("get_settings"),
  setSetting: (key: string, value: string) => invoke<void>("set_setting", { key, value }),
  listBusinessProfiles: () => invoke<BusinessProfile[]>("list_business_profiles"),
};
