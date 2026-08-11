import { useEffect } from "react";
import { useSettingsStore } from "@/lib/stores/settings";
import type { AppTheme } from "@/lib/types";

function applyTheme(theme: AppTheme) {
  const root = document.documentElement;
  root.classList.remove("light", "dark");

  if (theme === "system") {
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    root.classList.add(prefersDark ? "dark" : "light");
  } else {
    root.classList.add(theme === "dark" ? "dark" : "light");
  }
}

export function useTheme() {
  const theme = useSettingsStore((s) => s.settings.theme ?? "dark");
  const updateSettings = useSettingsStore((s) => s.updateSettings);

  useEffect(() => {
    applyTheme(theme);

    if (theme === "system") {
      const mq = window.matchMedia("(prefers-color-scheme: dark)");
      const handler = () => applyTheme("system");
      mq.addEventListener("change", handler);
      return () => mq.removeEventListener("change", handler);
    }
  }, [theme]);

  const setTheme = (t: AppTheme) => {
    updateSettings({ theme: t });
  };

  return { theme, setTheme } as const;
}
