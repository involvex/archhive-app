import { Link, useLocation, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import {
  Home,
  Compass,
  Library,
  Download,
  Settings,
  Puzzle,
  Radio,
  Sun,
  Moon,
  Monitor,
  Copy,
} from "lucide-react";
import { resolveAppVersion } from "@/lib/appVersion";
import { getPluginNavItems } from "@/lib/plugins/loader";
import { cn } from "@/lib/utils";
import { useKeyboardShortcuts } from "@/lib/hooks/useKeyboardShortcuts";
import { registerDefaultShortcuts } from "@/lib/shortcuts/defaults";
import { ShortcutBadge } from "@/components/ui/shortcut-badge";
import { CommandPalette } from "@/components/CommandPalette";
import { ShortcutHelp } from "@/components/ShortcutHelp";
import { api } from "@/lib/api/client";
import { useTheme } from "@/lib/hooks/useTheme";
import type { AppTheme } from "@/lib/types";

const desktopNavItems = [
  { to: "/", label: "Home", icon: Home, shortcut: "Ctrl+1" },
  { to: "/browse", label: "Browse", icon: Compass, shortcut: "Ctrl+2" },
  { to: "/library", label: "Library", icon: Library, shortcut: "Ctrl+3" },
  { to: "/live", label: "Live", icon: Radio, shortcut: "Ctrl+4" },
  { to: "/downloads", label: "Downloads", icon: Download, shortcut: "Ctrl+5" },
  { to: "/duplicates", label: "Duplicates", icon: Copy, shortcut: "Ctrl+7" },
  { to: "/settings", label: "Settings", icon: Settings, shortcut: "Ctrl+6" },
] as const;

const mobileNavItems = [
  { to: "/", label: "Home", icon: Home },
  { to: "/browse", label: "Browse", icon: Compass },
  { to: "/library", label: "Library", icon: Library },
  { to: "/live", label: "Live", icon: Radio },
  { to: "/downloads", label: "Downloads", icon: Download },
  { to: "/settings", label: "Settings", icon: Settings },
] as const;

const pluginNavItems = getPluginNavItems().map((item) => ({
  to: item.to,
  label: item.label,
  icon: Puzzle,
}));

export function AppShell({ children }: { children: React.ReactNode }) {
  const navigate = useNavigate();
  const location = useLocation();
  const desktopNav = [...desktopNavItems, ...pluginNavItems];
  const mobileNav = [...mobileNavItems, ...pluginNavItems];
  const [appVersion, setAppVersion] = useState("");
  const [sceneCount, setSceneCount] = useState<number | null>(null);
  // Q14: duplicate-group badge (null = unknown/failed, hidden).
  const [duplicateCount, setDuplicateCount] = useState<number | null>(null);
  const { theme, setTheme } = useTheme();

  const themeOptions: { value: AppTheme; icon: typeof Sun; label: string }[] = [
    { value: "dark", icon: Moon, label: "Dark" },
    { value: "light", icon: Sun, label: "Light" },
    { value: "system", icon: Monitor, label: "System" },
  ];

  useEffect(() => {
    void resolveAppVersion().then(setAppVersion);
  }, []);

  useEffect(() => {
    void api
      .getLibraryStats()
      .then((s) => setSceneCount(s.scene_count))
      .catch(() => {});
    // Best-effort badge; failures (e.g. remote not configured) stay hidden.
    void api
      .findDuplicates()
      .then((groups) => setDuplicateCount(groups.length))
      .catch(() => {});
  }, []);

  useEffect(() => {
    registerDefaultShortcuts((path: string) => navigate({ to: path }));
  }, [navigate]);

  useKeyboardShortcuts();

  return (
    <div className="flex min-h-screen max-w-[100vw] overflow-x-hidden">
      <aside className="hidden md:flex w-56 shrink-0 flex-col border-r border-[var(--color-border)] bg-[var(--color-card)] p-4">
        <div className="mb-6 px-2">
          <h1 className="text-lg font-bold tracking-tight">ArcHive</h1>
          <p className="text-xs text-[var(--color-muted-foreground)]">
            Browse · Download · Library
          </p>
        </div>
        <nav className="flex flex-col gap-1">
          {desktopNav.map(({ to, label, icon: Icon, ...rest }) => {
            const count =
              to === "/library" && sceneCount != null && sceneCount > 0
                ? sceneCount > 99
                  ? "99+"
                  : sceneCount
                : to === "/duplicates" && duplicateCount != null && duplicateCount > 0
                  ? duplicateCount > 99
                    ? "99+"
                    : duplicateCount
                  : null;
            return (
              <Link
                key={to}
                to={to}
                className={cn(
                  "flex items-center justify-between rounded-md px-3 py-2 text-sm transition-colors",
                  "hover:bg-[var(--color-accent)]",
                  "[&.active]:bg-[var(--color-primary)] [&.active]:text-[var(--color-primary-foreground)]",
                )}
              >
                <span className="flex items-center gap-2">
                  <Icon className="h-4 w-4" />
                  {label}
                  {count != null && (
                    <span className="rounded-full bg-[var(--color-muted)] px-1.5 py-0.5 text-[10px] font-medium leading-none">
                      {count}
                    </span>
                  )}
                </span>
                {"shortcut" in rest && rest.shortcut && (
                  <ShortcutBadge keys={rest.shortcut as string} className="opacity-50" />
                )}
              </Link>
            );
          })}
        </nav>
        <div className="mt-4 flex items-center gap-1 px-2">
          {themeOptions.map(({ value, icon: Icon, label }) => (
            <button
              key={value}
              type="button"
              onClick={() => setTheme(value)}
              title={label}
              className={cn(
                "rounded-md p-1.5 transition-colors",
                theme === value
                  ? "bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
                  : "text-[var(--color-muted-foreground)] hover:bg-[var(--color-accent)]",
              )}
            >
              <Icon className="h-3.5 w-3.5" />
            </button>
          ))}
        </div>
        {appVersion && (
          <p className="mt-auto px-2 pt-4 text-[10px] text-[var(--color-muted-foreground)]">
            v{appVersion}
          </p>
        )}
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <main className="flex-1 overflow-x-hidden p-4 pb-24 md:p-6 md:pb-6">{children}</main>
        <nav className="md:hidden fixed bottom-0 left-0 right-0 z-40 border-t border-[var(--color-border)] bg-[var(--color-card)] pb-[env(safe-area-inset-bottom)]">
          <div className="flex justify-around pt-2 pb-1">
            {mobileNav.map(({ to, label, icon: Icon }) => {
              const isActive = location.pathname === to || location.pathname.startsWith(to + "/");
              return (
                <Link
                  key={to}
                  to={to}
                  className={cn(
                    "flex min-w-0 flex-1 flex-col items-center gap-1 px-2 py-2 text-[11px] font-medium transition-colors",
                    isActive
                      ? "text-[var(--color-primary)]"
                      : "text-[var(--color-muted-foreground)]",
                  )}
                >
                  <div className="relative">
                    <Icon className="h-6 w-6 shrink-0" />
                    <span
                      className={cn(
                        "absolute -bottom-1.5 left-1/2 -translate-x-1/2 h-0.5 w-4 rounded-full transition-colors",
                        isActive ? "bg-[var(--color-primary)]" : "bg-transparent",
                      )}
                    />
                  </div>
                  <span className="truncate">{label}</span>
                </Link>
              );
            })}
          </div>
        </nav>
      </div>

      <CommandPalette />
      <ShortcutHelp />
    </div>
  );
}
