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

export type LookupKind = "category" | "brand";

export interface LookupItem {
  id: string;
  nameAr: string;
  nameEn: string;
  isActive: boolean;
  productCount: number;
}

export interface LookupInput {
  id?: string;
  nameAr: string;
  nameEn: string;
  isActive: boolean;
}

export interface Unit {
  id: string;
  code: string;
  nameAr: string;
  nameEn: string;
  allowsDecimal: boolean;
  isSystem: boolean;
}

export interface ProductFilter {
  query?: string;
  categoryId?: string;
  brandId?: string;
  includeArchived?: boolean;
  limit?: number;
  offset?: number;
}

export interface ProductListItem {
  id: string;
  sku: string | null;
  nameAr: string;
  nameEn: string;
  categoryName: string | null;
  brandName: string | null;
  baseUnitName: string;
  /** بالقروش */
  purchasePrice: number;
  /** بالقروش */
  salePrice: number;
  /** بالألف */
  minStockQty: number;
  isActive: boolean;
  archived: boolean;
  variantCount: number;
  firstBarcode: string | null;
}

export interface VariantDetail {
  id: string;
  nameAr: string;
  nameEn: string;
  sku: string | null;
  purchasePrice: number;
  salePrice: number;
  isActive: boolean;
  barcodes: string[];
}

export interface ProductUnitDetail {
  unitId: string;
  /** بالمليون */
  factorMicro: number;
  isPurchaseUnit: boolean;
  isSaleUnit: boolean;
}

export interface ProductDetail {
  id: string;
  sku: string | null;
  nameAr: string;
  nameEn: string;
  categoryId: string | null;
  brandId: string | null;
  baseUnitId: string;
  purchasePrice: number;
  salePrice: number;
  minStockQty: number;
  isWeighted: boolean;
  trackExpiry: boolean;
  isActive: boolean;
  archived: boolean;
  hasImage: boolean;
  barcodes: string[];
  variants: VariantDetail[];
  units: ProductUnitDetail[];
}

/** الأرقام تُرسل كنصوص ويحلّلها Rust. */
export interface ProductInput {
  id?: string;
  sku?: string | null;
  nameAr: string;
  nameEn: string;
  categoryId: string | null;
  brandId: string | null;
  baseUnitId: string;
  purchasePrice: string;
  salePrice: string;
  minStockQty: string;
  isWeighted: boolean;
  trackExpiry: boolean;
  isActive: boolean;
  barcodes: string[];
  variants: {
    id?: string;
    nameAr: string;
    nameEn: string;
    sku?: string | null;
    purchasePrice: string;
    salePrice: string;
    isActive: boolean;
    barcodes: string[];
  }[];
  units: { unitId: string; factor: string; isPurchaseUnit: boolean; isSaleUnit: boolean }[];
}

/** كل الحسابات والتحقق في Rust. هذه الطبقة مجرد جسر للأوامر. */
export const api = {
  getAppInfo: () => invoke<AppInfo>("get_app_info"),
  getDbHealth: () => invoke<DbHealth>("get_db_health"),
  getSettings: () => invoke<Record<string, string>>("get_settings"),
  setSetting: (key: string, value: string) => invoke<void>("set_setting", { key, value }),
  listBusinessProfiles: () => invoke<BusinessProfile[]>("list_business_profiles"),
  listLookups: (kind: LookupKind) => invoke<LookupItem[]>("list_lookups", { kind }),
  saveLookup: (kind: LookupKind, input: LookupInput) => invoke<string>("save_lookup", { kind, input }),
  listUnits: () => invoke<Unit[]>("list_units"),
  createUnit: (input: { nameAr: string; nameEn: string; allowsDecimal: boolean }) =>
    invoke<string>("create_unit", { input }),
  listProducts: (filter: ProductFilter) => invoke<ProductListItem[]>("list_products", { filter }),
  getProduct: (id: string) => invoke<ProductDetail>("get_product", { id }),
  saveProduct: (input: ProductInput) => invoke<string>("save_product", { input }),
  setProductArchived: (id: string, archived: boolean) => invoke<void>("set_product_archived", { id, archived }),
  setProductImage: (id: string, dataBase64: string) => invoke<void>("set_product_image", { id, dataBase64 }),
  removeProductImage: (id: string) => invoke<void>("remove_product_image", { id }),
  getProductImage: (id: string) => invoke<string | null>("get_product_image", { id }),
};
