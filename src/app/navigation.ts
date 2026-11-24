import {
  BarChart3,
  Building2,
  LayoutDashboard,
  Package,
  Receipt,
  Settings,
  ShoppingCart,
  Tags,
  Truck,
  UserCog,
  Users,
  Wallet,
  Warehouse,
  type LucideIcon,
} from "lucide-react";

export interface NavItem {
  path: string;
  label: string;
  icon: LucideIcon;
  /** رقم المرحلة التي يتم فيها تنفيذ الصفحة (0 = جاهزة الآن) */
  phase: number;
}

export const navItems: NavItem[] = [
  { path: "/", label: "لوحة التحكم", icon: LayoutDashboard, phase: 0 },
  { path: "/pos", label: "الكاشير", icon: ShoppingCart, phase: 2 },
  { path: "/products", label: "المنتجات", icon: Package, phase: 1 },
  { path: "/categories", label: "التصنيفات", icon: Tags, phase: 1 },
  { path: "/sales", label: "المبيعات", icon: Receipt, phase: 2 },
  { path: "/inventory", label: "المخزون", icon: Warehouse, phase: 3 },
  { path: "/purchases", label: "المشتريات", icon: Truck, phase: 3 },
  { path: "/suppliers", label: "الموردون", icon: Building2, phase: 3 },
  { path: "/customers", label: "العملاء", icon: Users, phase: 4 },
  { path: "/expenses", label: "المصروفات والخزينة", icon: Wallet, phase: 4 },
  { path: "/reports", label: "التقارير", icon: BarChart3, phase: 5 },
  { path: "/users", label: "المستخدمون والصلاحيات", icon: UserCog, phase: 6 },
  { path: "/settings", label: "الإعدادات", icon: Settings, phase: 0 },
];
