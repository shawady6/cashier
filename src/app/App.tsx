import { useEffect } from "react";
import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import ComingSoon from "../components/ComingSoon";
import MainLayout from "../layouts/MainLayout";
import CategoriesPage from "../pages/Categories";
import CustomersPage from "../pages/Customers";
import Dashboard from "../pages/Dashboard";
import TreasuryPage from "../pages/Treasury";
import InventoryPage from "../pages/Inventory";
import PosPage from "../pages/POS";
import ProductsPage from "../pages/Products";
import PurchasesPage from "../pages/Purchases";
import ReportsPage from "../pages/Reports";
import SalesPage from "../pages/Sales";
import SettingsPage from "../pages/Settings";
import SuppliersPage from "../pages/Suppliers";
import { useSettingsStore } from "../store/settingsStore";
import { navItems } from "./navigation";

export default function App() {
  const load = useSettingsStore((s) => s.load);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <HashRouter>
      <Routes>
        <Route element={<MainLayout />}>
          <Route index element={<Dashboard />} />
          <Route path="pos" element={<PosPage />} />
          <Route path="sales" element={<SalesPage />} />
          <Route path="products" element={<ProductsPage />} />
          <Route path="categories" element={<CategoriesPage />} />
          <Route path="inventory" element={<InventoryPage />} />
          <Route path="purchases" element={<PurchasesPage />} />
          <Route path="suppliers" element={<SuppliersPage />} />
          <Route path="customers" element={<CustomersPage />} />
          <Route path="expenses" element={<TreasuryPage />} />
          <Route path="reports" element={<ReportsPage />} />
          <Route path="settings" element={<SettingsPage />} />
          {navItems
            .filter((item) => item.phase > 5)
            .map((item) => (
              <Route
                key={item.path}
                path={item.path}
                element={<ComingSoon title={item.label} phase={item.phase} />}
              />
            ))}
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </HashRouter>
  );
}
