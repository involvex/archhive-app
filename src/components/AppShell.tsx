import { Link } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import {
  Home,
  Compass,
  Library,
  Download,
  Users,
  Tags,
  Settings,
  Puzzle,
  FolderOpen,
} from "lucide-react";
import { resolveAppVersion } from "@/lib/appVersion";
import { getPluginNavItems } from "@/lib/plugins/loader";
import { cn } from "@/lib/utils";

const desktopNavItems = [
  { to: "/", label: "Home", icon: Home },
  { to: "/browse", label: "Browse", icon: Compass },
  { to: "/library/scenes", label: "Library", icon: Library },
  { to: "/files", label: "Files", icon: FolderOpen },
  { to: "/downloads", label: "Downloads", icon: Download },
  { to: "/library/performers", label: "Performers", icon: Users },
  { to: "/library/tags", label: "Tags", icon: Tags },
  { to: "/settings", label: "Settings", icon: Settings },
] as const;

const mobileNavItems = [
  { to: "/", label: "Home", icon: Home },
  { to: "/browse", label: "Browse", icon: Compass },
  { to: "/downloads", label: "Downloads", icon: Download },
  { to: "/library/scenes", label: "Library", icon: Library },
  { to: "/files", label: "Files", icon: FolderOpen },
  { to: "/settings", label: "Settings", icon: Settings },
] as const;

const pluginNavItems = getPluginNavItems().map((item) => ({
  to: item.to,
  label: item.label,
  icon: Puzzle,
}));

export function AppShell({ children }: { children: React.ReactNode }) {
  const desktopNav = [...desktopNavItems, ...pluginNavItems];
  const mobileNav = [...mobileNavItems, ...pluginNavItems];
  const [appVersion, setAppVersion] = useState("");

  useEffect(() => {
    void resolveAppVersion().then(setAppVersion);
  }, []);

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
          {desktopNav.map(({ to, label, icon: Icon }) => (
            <Link
              key={to}
              to={to}
              className={cn(
                "flex items-center gap-2 rounded-md px-3 py-2 text-sm transition-colors",
                "hover:bg-[var(--color-accent)]",
                "[&.active]:bg-[var(--color-primary)] [&.active]:text-[var(--color-primary-foreground)]",
              )}
            >
              <Icon className="h-4 w-4" />
              {label}
            </Link>
          ))}
        </nav>
        {appVersion && (
          <p className="mt-auto px-2 pt-4 text-[10px] text-[var(--color-muted-foreground)]">
            v{appVersion}
          </p>
        )}
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <main className="flex-1 overflow-x-hidden p-4 pb-24 md:p-6 md:pb-6">{children}</main>
        <nav className="md:hidden fixed bottom-0 left-0 right-0 z-40 border-t border-[var(--color-border)] bg-[var(--color-card)] pb-[env(safe-area-inset-bottom)]">
          <div className="flex justify-around py-2">
            {mobileNav.map(({ to, label, icon: Icon }) => (
              <Link
                key={to}
                to={to}
                className="flex min-w-0 flex-1 flex-col items-center gap-0.5 px-1 text-[10px] text-[var(--color-muted-foreground)] [&.active]:text-[var(--color-primary)]"
              >
                <Icon className="h-5 w-5 shrink-0" />
                <span className="truncate">{label}</span>
              </Link>
            ))}
          </div>
        </nav>
      </div>
    </div>
  );
}
