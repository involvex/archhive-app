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

/** Parse "HH:MM" into minutes-since-midnight. */
function parseTimeHM(time: string): number | null {
  const [h, m] = time.split(":").map(Number);
  if (Number.isNaN(h) || Number.isNaN(m)) return null;
  return h * 60 + m;
}

/** Returns "dark" if current time is in the scheduled dark window, "light" otherwise. */
function scheduledTheme(from: string, to: string): "dark" | "light" {
  const now = new Date();
  const current = now.getHours() * 60 + now.getMinutes();
  const fromMin = parseTimeHM(from);
  const toMin = parseTimeHM(to);
  if (fromMin == null || toMin == null) return "dark";

  if (fromMin <= toMin) {
    return current >= fromMin && current < toMin ? "dark" : "light";
  }
  return current >= fromMin || current < toMin ? "dark" : "light";
}

export function useTheme() {
  const settings = useSettingsStore((s) => s.settings);
  const theme = settings.theme ?? "dark";
  const updateSettings = useSettingsStore((s) => s.updateSettings);

  useEffect(() => {
    applyTheme(theme);

    if (theme === "system") {
      const mq = window.matchMedia("(prefers-color-scheme: dark)");
      const handler = () => applyTheme("system");
      mq.addEventListener("change", handler);
      return () => mq.removeEventListener("change", handler);
    }

    if (theme === "scheduled") {
      const tick = () => {
        const resolved = scheduledTheme(
          settings.theme_schedule_from ?? "19:00",
          settings.theme_schedule_to ?? "07:00",
        );
        document.documentElement.classList.remove("light", "dark");
        document.documentElement.classList.add(resolved);
      };
      tick();
      const interval = setInterval(tick, 60_000);
      return () => clearInterval(interval);
    }
  }, [theme, settings.theme_schedule_from, settings.theme_schedule_to]);

  const setTheme = (t: AppTheme) => {
    updateSettings({ theme: t });
  };

  return { theme, setTheme } as const;
}
