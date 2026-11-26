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
  /** بالألف، بالوحدة الأساسية */
  stockQty: number;
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
  /** سعر بيع الوحدة الكبيرة بالقروش. null = سعر القطعة × العدد */
  salePriceMinor: number | null;
  barcode: string | null;
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
  units: { unitId: string; factor: string; isPurchaseUnit: boolean; isSaleUnit: boolean; salePrice?: string; barcode?: string }[];
}

export interface SellUnit {
  unitId: string;
  nameAr: string;
  factorMicro: number;
  allowsDecimal: boolean;
  /** سعر بيع هذه الوحدة. null = سعر القطعة × المعامل */
  salePriceMinor: number | null;
}

export interface Sellable {
  productId: string;
  variantId: string | null;
  nameAr: string;
  variantName: string | null;
  sku: string | null;
  barcode: string | null;
  salePrice: number;
  stockQty: number;
  baseUnitId: string;
  baseUnitName: string;
  allowsDecimal: boolean;
  units: SellUnit[];
  /** وحدة باركود الوحدة الكبيرة لو طابقه البحث */
  matchedUnitId: string | null;
}

export interface DiscountInput {
  kind: "amount" | "percent";
  value: string;
}

export interface CartItemInput {
  productId: string;
  variantId: string | null;
  unitId: string | null;
  qty: string;
  discount: DiscountInput | null;
}

export interface PaymentInput {
  method: "cash" | "card";
  amount: string;
  reference?: string;
}

export interface CartInput {
  items: CartItemInput[];
  invoiceDiscount: DiscountInput | null;
  payments: PaymentInput[];
  clientRequestId?: string;
  note?: string;
  customerId?: string | null;
}

export interface QuoteLine {
  productId: string;
  variantId: string | null;
  nameAr: string;
  variantName: string | null;
  unitName: string;
  qtyMilli: number;
  unitPrice: number;
  gross: number;
  discount: number;
  invoiceDiscount: number;
  tax: number;
  total: number;
}

export interface CartQuote {
  lines: QuoteLine[];
  subtotal: number;
  itemDiscount: number;
  invoiceDiscount: number;
  tax: number;
  total: number;
  paid: number;
  change: number;
  remaining: number;
  cashNet: number;
  card: number;
}

export interface CheckoutResult {
  saleId: string;
  invoiceNo: string;
  total: number;
  change: number;
  duplicate: boolean;
}

export interface SaleListItem {
  id: string;
  invoiceNo: string;
  createdAt: string;
  totalMinor: number;
  paymentStatus: string;
  itemCount: number;
  returnedMinor: number;
  customerName: string | null;
}

export interface SaleItemDetail {
  id: string;
  lineNo: number;
  nameAr: string;
  variantName: string | null;
  unitName: string;
  qtyMilli: number;
  unitPriceMinor: number;
  grossMinor: number;
  discountMinor: number;
  invoiceDiscountMinor: number;
  taxMinor: number;
  totalMinor: number;
  returnedQtyMilli: number;
}

export interface SaleDetail {
  id: string;
  invoiceNo: string;
  createdAt: string;
  paymentStatus: string;
  subtotalMinor: number;
  itemDiscountMinor: number;
  invoiceDiscountMinor: number;
  taxMinor: number;
  totalMinor: number;
  paidMinor: number;
  changeMinor: number;
  note: string;
  customerName: string | null;
  dueMinor: number;
  items: SaleItemDetail[];
  payments: { method: string; amountMinor: number; reference: string }[];
  returns: { id: string; returnNo: string; refundMinor: number; refundMethod: string; reason: string; createdAt: string; creditMinor: number }[];
}

export interface SuspendedSale {
  id: string;
  label: string;
  payload: string;
  createdAt: string;
}

export interface ReturnInput {
  saleId: string;
  items: { saleItemId: string; qty: string }[];
  reason: string;
  refundMethod: "cash" | "card";
}

export interface AdjustInput {
  productId: string;
  variantId: string | null;
  direction: "increase" | "decrease";
  qty: string;
  unitId?: string | null;
  /** عدد القطع في الوحدة لهذه العملية فقط */
  factor?: string;
  unitCost?: string;
  salePrice?: string;
  note?: string;
}

export interface Warehouse { id: string; nameAr: string; isDefault: boolean; isActive: boolean; stockValueMinor: number }
export interface BalanceRow {
  warehouseId: string; warehouseName: string; productId: string; variantId: string | null;
  nameAr: string; variantName: string | null; sku: string | null; baseUnitName: string;
  qtyMilli: number; valueMinor: number; avgCostMinor: number;
}
export interface BalancesReport { rows: BalanceRow[]; totalValueMinor: number }
export interface MovementRow {
  id: string; createdAt: string; warehouseName: string; nameAr: string; variantName: string | null;
  movementType: string; qtyDeltaMilli: number; valueDeltaMinor: number; note: string;
}
export interface LowStockRow { productId: string; nameAr: string; baseUnitName: string; stockQtyMilli: number; minStockQtyMilli: number }
export interface TransferRow { id: string; transferNo: string; fromName: string; toName: string; note: string; createdAt: string; itemCount: number; valueMinor: number }
export interface TransferInput {
  fromWarehouseId: string; toWarehouseId: string; note?: string;
  items: { productId: string; variantId: string | null; qty: string }[];
}
export interface CountRow { id: string; countNo: string; warehouseName: string; status: string; note: string; createdAt: string; finalizedAt: string | null; itemCount: number }
export interface CountItem { id: string; nameAr: string; variantName: string | null; baseUnitName: string; systemQtyMilli: number; countedQtyMilli: number | null }
export interface CountDetail { header: CountRow; items: CountItem[] }
export interface Supplier { id: string; nameAr: string; phone: string; address: string; notes: string; isActive: boolean; balanceMinor: number }
export interface SupplierInput { id?: string; nameAr: string; phone: string; address: string; notes: string; isActive: boolean }
export interface PurchaseRow {
  id: string; purchaseNo: string; supplierName: string; supplierInvoiceNo: string; status: string;
  totalMinor: number; paidMinor: number; returnedMinor: number; createdAt: string; receivedAt: string | null;
}
export interface PurchaseItemDetail {
  id: string; lineNo: number; productId: string; variantId: string | null; nameAr: string; variantName: string | null;
  unitId: string; unitName: string; unitFactorMicro: number; qtyMilli: number; baseQtyMilli: number; unitCostMinor: number; lineTotalMinor: number; netMinor: number;
  newSalePriceMinor: number | null; returnedQtyMilli: number;
}
export interface PurchaseDetail {
  header: PurchaseRow; supplierId: string; warehouseId: string; warehouseName: string; note: string;
  subtotalMinor: number; discountMinor: number; outstandingMinor: number;
  items: PurchaseItemDetail[];
  payments: { id: string; method: string; amountMinor: number; note: string; createdAt: string }[];
  returns: { id: string; returnNo: string; totalMinor: number; note: string; createdAt: string }[];
}
export interface PurchaseInput {
  id?: string; supplierId: string; warehouseId: string; supplierInvoiceNo: string; note: string;
  discount: DiscountInput | null;
  items: { productId: string; variantId: string | null; unitId: string | null; qty: string; unitCost: string; factor?: string; salePrice?: string }[];
}

export interface SessionSummary {
  id: string; status: string; openedAt: string; closedAt: string | null; openingMinor: number;
  cashSalesMinor: number; cardSalesMinor: number; cashReturnsMinor: number; cashExpensesMinor: number;
  supplierPaymentsMinor: number; customerPaymentsMinor: number; cashInMinor: number; cashOutMinor: number;
  cardTotalMinor: number; expectedCashMinor: number; countedCashMinor: number | null; differenceMinor: number | null; note: string;
}
export interface ExpenseCategory { id: string; nameAr: string; isActive: boolean }
export interface ExpenseRow {
  id: string; categoryName: string; amountMinor: number; method: string; description: string;
  createdAt: string; voidedAt: string | null; voidReason: string | null;
}
export interface ExpenseReport { rows: ExpenseRow[]; totalMinor: number }
export interface Customer {
  id: string; nameAr: string; phone: string; address: string; notes: string;
  creditLimitMinor: number; isActive: boolean; balanceMinor: number;
}
export interface CustomerInput {
  id?: string; nameAr: string; phone: string; address: string; notes: string; creditLimit: string; isActive: boolean;
}
export interface StatementRow { date: string; kind: "sale" | "payment" | "return"; refNo: string; debitMinor: number; creditMinor: number; balanceMinor: number }
export interface Statement { customer: Customer; rows: StatementRow[] }

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
  searchSellables: (query: string) => invoke<Sellable[]>("search_sellables", { query }),
  priceCart: (cart: CartInput) => invoke<CartQuote>("price_cart", { cart }),
  checkout: (cart: CartInput) => invoke<CheckoutResult>("checkout", { cart }),
  listSales: (filter: { query?: string; dateFrom?: string; dateTo?: string }) =>
    invoke<SaleListItem[]>("list_sales", { filter }),
  getSale: (id: string) => invoke<SaleDetail>("get_sale", { id }),
  createSalesReturn: (input: ReturnInput) =>
    invoke<{ returnId: string; returnNo: string; refund: number }>("create_sales_return", { input }),
  suspendSale: (label: string, payload: string) => invoke<string>("suspend_sale", { label, payload }),
  listSuspendedSales: () => invoke<SuspendedSale[]>("list_suspended_sales"),
  resumeSale: (id: string) => invoke<SuspendedSale>("resume_sale", { id }),
  discardSuspendedSale: (id: string) => invoke<void>("discard_suspended_sale", { id }),
  adjustStock: (input: AdjustInput) => invoke<void>("adjust_stock", { input }),
  listWarehouses: () => invoke<Warehouse[]>("list_warehouses"),
  saveWarehouse: (input: { id?: string; nameAr: string; isActive: boolean; isDefault: boolean }) =>
    invoke<string>("save_warehouse", { input }),
  stockBalances: (filter: { warehouseId?: string; query?: string }) => invoke<BalancesReport>("stock_balances", { filter }),
  stockMovements: (filter: { warehouseId?: string; movementType?: string; limit?: number }) =>
    invoke<MovementRow[]>("stock_movements", { filter }),
  lowStockReport: () => invoke<LowStockRow[]>("low_stock_report"),
  createTransfer: (input: TransferInput) => invoke<string>("create_transfer", { input }),
  listTransfers: () => invoke<TransferRow[]>("list_transfers"),
  createCount: (input: { warehouseId: string; categoryId?: string | null; note?: string }) => invoke<string>("create_count", { input }),
  listCounts: () => invoke<CountRow[]>("list_counts"),
  getCount: (id: string) => invoke<CountDetail>("get_count", { id }),
  saveCountLines: (id: string, lines: { itemId: string; countedQty: string | null }[]) => invoke<void>("save_count_lines", { id, lines }),
  finalizeCount: (id: string) => invoke<number>("finalize_count", { id }),
  cancelCount: (id: string) => invoke<void>("cancel_count", { id }),
  listSuppliers: () => invoke<Supplier[]>("list_suppliers"),
  saveSupplier: (input: SupplierInput) => invoke<string>("save_supplier", { input }),
  listPurchases: (filter: { supplierId?: string; status?: string; query?: string }) => invoke<PurchaseRow[]>("list_purchases", { filter }),
  getPurchase: (id: string) => invoke<PurchaseDetail>("get_purchase", { id }),
  savePurchaseDraft: (input: PurchaseInput) => invoke<string>("save_purchase_draft", { input }),
  receivePurchase: (id: string) => invoke<void>("receive_purchase", { id }),
  cancelPurchaseDraft: (id: string) => invoke<void>("cancel_purchase_draft", { id }),
  addSupplierPayment: (input: { purchaseId: string; method: "cash" | "card"; amount: string; note?: string }) =>
    invoke<string>("add_supplier_payment", { input }),
  createPurchaseReturn: (input: { purchaseId: string; items: { purchaseItemId: string; qty: string }[]; note?: string }) =>
    invoke<{ returnNo: string; credit: number }>("create_purchase_return", { input }),
  openCashSession: (opening: string, note?: string) => invoke<string>("open_cash_session", { opening, note }),
  currentCashSession: () => invoke<SessionSummary | null>("current_cash_session"),
  closeCashSession: (id: string, counted: string, note?: string) => invoke<SessionSummary>("close_cash_session", { id, counted, note }),
  listCashSessions: () => invoke<SessionSummary[]>("list_cash_sessions"),
  addCashEntry: (input: { direction: "in" | "out"; amount: string; reason: string }) => invoke<string>("add_cash_entry", { input }),
  listExpenseCategories: () => invoke<ExpenseCategory[]>("list_expense_categories"),
  saveExpenseCategory: (input: { id?: string; nameAr: string; isActive: boolean }) => invoke<string>("save_expense_category", { input }),
  createExpense: (input: { categoryId: string; amount: string; method: "cash" | "card"; description: string }) =>
    invoke<string>("create_expense", { input }),
  voidExpense: (id: string, reason: string) => invoke<void>("void_expense", { id, reason }),
  listExpenses: (filter: { categoryId?: string; dateFrom?: string; dateTo?: string }) => invoke<ExpenseReport>("list_expenses", { filter }),
  listCustomers: () => invoke<Customer[]>("list_customers"),
  saveCustomer: (input: CustomerInput) => invoke<string>("save_customer", { input }),
  addCustomerPayment: (input: { customerId: string; method: "cash" | "card"; amount: string; note?: string }) =>
    invoke<string>("add_customer_payment", { input }),
  customerStatement: (id: string) => invoke<Statement>("customer_statement", { id }),
};
