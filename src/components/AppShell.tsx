import { Link } from "@tanstack/react-router";
import { Home, Compass, Library, Download, Users, Tags, Settings } from "lucide-react";
import { cn } from "@/lib/utils";

const navItems = [
  { to: "/", label: "Home", icon: Home },
  { to: "/browse", label: "Browse", icon: Compass },
  { to: "/library/scenes", label: "Library", icon: Library },
  { to: "/downloads", label: "Downloads", icon: Download },
  { to: "/library/performers", label: "Performers", icon: Users },
  { to: "/library/tags", label: "Tags", icon: Tags },
  { to: "/settings", label: "Settings", icon: Settings },
] as const;

export function AppShell({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex min-h-screen">
      <aside className="hidden md:flex w-56 flex-col border-r border-[var(--color-border)] bg-[var(--color-card)] p-4">
        <div className="mb-6 px-2">
          <h1 className="text-lg font-bold tracking-tight">Scrawler</h1>
          <p className="text-xs text-[var(--color-muted-foreground)]">
            Browse · Download · Library
          </p>
        </div>
        <nav className="flex flex-col gap-1">
          {navItems.map(({ to, label, icon: Icon }) => (
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
      </aside>

      <div className="flex flex-1 flex-col">
        <main className="flex-1 p-4 md:p-6 pb-20 md:pb-6">{children}</main>
        <nav className="md:hidden fixed bottom-0 left-0 right-0 border-t border-[var(--color-border)] bg-[var(--color-card)] flex justify-around py-2">
          {navItems.slice(0, 5).map(({ to, label, icon: Icon }) => (
            <Link
              key={to}
              to={to}
              className="flex flex-col items-center gap-0.5 px-2 text-[10px] text-[var(--color-muted-foreground)] [&.active]:text-[var(--color-primary)]"
            >
              <Icon className="h-5 w-5" />
              {label}
            </Link>
          ))}
        </nav>
      </div>
    </div>
  );
}
