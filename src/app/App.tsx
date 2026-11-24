import { useEffect } from "react";
import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import ComingSoon from "../components/ComingSoon";
import MainLayout from "../layouts/MainLayout";
import Dashboard from "../pages/Dashboard";
import SettingsPage from "../pages/Settings";
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
          <Route path="settings" element={<SettingsPage />} />
          {navItems
            .filter((item) => item.phase !== 0)
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
