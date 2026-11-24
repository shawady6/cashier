import { Moon, Sun } from "lucide-react";
import { NavLink, Outlet, useLocation } from "react-router-dom";
import { navItems } from "../app/navigation";
import { useSettingsStore } from "../store/settingsStore";

export default function MainLayout() {
  const { pathname } = useLocation();
  const theme = useSettingsStore((s) => s.settings.theme ?? "light");
  const update = useSettingsStore((s) => s.update);
  const current = navItems.find((i) => i.path === pathname);

  return (
    <div className="flex h-full">
      <aside className="flex w-64 shrink-0 flex-col border-e border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-800">
        <div className="px-5 py-5 text-lg font-bold text-navy-900 dark:text-white">
          نظام نقاط البيع
        </div>
        <nav className="flex-1 space-y-1 overflow-y-auto px-3 pb-4">
          {navItems.map((item) => (
            <NavLink
              key={item.path}
              to={item.path}
              end={item.path === "/"}
              className={({ isActive }) =>
                `flex items-center gap-3 rounded-lg px-3 py-2 text-sm ${
                  isActive
                    ? "bg-teal-50 font-semibold text-teal-700 dark:bg-teal-900/40 dark:text-teal-200"
                    : "text-slate-600 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-700"
                }`
              }
            >
              <item.icon size={18} aria-hidden />
              {item.label}
            </NavLink>
          ))}
        </nav>
      </aside>
      <div className="flex min-w-0 flex-1 flex-col">
        <header className="flex items-center justify-between border-b border-slate-200 bg-white px-6 py-3 dark:border-slate-700 dark:bg-slate-800">
          <h1 className="text-base font-semibold">{current?.label ?? "نظام نقاط البيع"}</h1>
          <button
            type="button"
            onClick={() => void update("theme", theme === "dark" ? "light" : "dark")}
            className="rounded-lg p-2 text-slate-600 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-700"
            aria-label={theme === "dark" ? "التبديل إلى الوضع الفاتح" : "التبديل إلى الوضع الداكن"}
          >
            {theme === "dark" ? <Sun size={18} /> : <Moon size={18} />}
          </button>
        </header>
        <main className="flex-1 overflow-y-auto p-6">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
