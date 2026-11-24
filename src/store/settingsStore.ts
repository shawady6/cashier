import { create } from "zustand";
import { api } from "../services/api";

function applyTheme(theme: string) {
  document.documentElement.classList.toggle("dark", theme === "dark");
}

interface SettingsState {
  settings: Record<string, string>;
  loaded: boolean;
  error: string | null;
  load: () => Promise<void>;
  update: (key: string, value: string) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: {},
  loaded: false,
  error: null,
  load: async () => {
    try {
      const settings = await api.getSettings();
      applyTheme(settings.theme ?? "light");
      set({ settings, loaded: true, error: null });
    } catch (e) {
      set({ error: String(e), loaded: true });
    }
  },
  update: async (key, value) => {
    await api.setSetting(key, value);
    if (key === "theme") applyTheme(value);
    set({ settings: { ...get().settings, [key]: value } });
  },
}));
