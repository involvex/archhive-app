import { useEffect, useState } from "react";
import { Lock } from "lucide-react";
import { api } from "@/lib/api/client";
import { authenticateForVault } from "@/lib/biometrics";
import { hasLocalBackend } from "@/lib/runtime";
import { useAppLockStore } from "@/lib/stores/appLock";
import { useSettingsStore } from "@/lib/stores/settings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

/**
 * Full-screen PIN gate for App Lock (R11). Mounted at the app root; listens for
 * background/foreground transitions via the Page Visibility API.
 */
export function AppLockGate() {
  const hydrated = useSettingsStore((s) => s.hydrated);
  const enabled = useSettingsStore((s) => s.settings.app_lock_enabled === true);
  const locked = useAppLockStore((s) => s.locked);
  const checkColdStart = useAppLockStore((s) => s.checkColdStart);
  const markHidden = useAppLockStore((s) => s.markHidden);
  const checkResume = useAppLockStore((s) => s.checkResume);
  const unlock = useAppLockStore((s) => s.unlock);

  const [pin, setPin] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!hydrated || !hasLocalBackend()) return;
    checkColdStart();
  }, [hydrated, checkColdStart]);

  useEffect(() => {
    if (!enabled && locked) unlock();
  }, [enabled, locked, unlock]);

  useEffect(() => {
    if (!enabled || !hasLocalBackend()) return;
    function onVisibility() {
      if (document.hidden) markHidden();
      else checkResume();
    }
    function onPageHide() {
      markHidden();
    }
    document.addEventListener("visibilitychange", onVisibility);
    window.addEventListener("pagehide", onPageHide);
    return () => {
      document.removeEventListener("visibilitychange", onVisibility);
      window.removeEventListener("pagehide", onPageHide);
    };
  }, [enabled, markHidden, checkResume]);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError("");
    setBusy(true);
    try {
      const bio = await authenticateForVault("Unlock ArcHive");
      if (!bio.ok) {
        setError(bio.message ?? "Authentication cancelled");
        return;
      }
      await api.verifyVaultPin(pin.trim());
      setPin("");
      unlock();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Incorrect PIN");
    } finally {
      setBusy(false);
    }
  }

  if (!hydrated || !hasLocalBackend() || !enabled || !locked) return null;

  return (
    <div
      className="fixed inset-0 z-[500] flex items-center justify-center bg-[var(--color-background)]/95 p-6 backdrop-blur-md"
      role="dialog"
      aria-modal="true"
      aria-labelledby="app-lock-title"
    >
      <form
        onSubmit={(e) => void submit(e)}
        className="w-full max-w-sm space-y-4 rounded-lg border border-[var(--color-border)] bg-[var(--color-card)] p-6 shadow-lg"
      >
        <div className="flex items-center gap-3">
          <div className="rounded-md bg-[var(--color-primary)]/15 p-2">
            <Lock className="h-5 w-5 text-[var(--color-primary)]" />
          </div>
          <div>
            <h2 id="app-lock-title" className="text-lg font-semibold">
              ArcHive is locked
            </h2>
            <p className="text-xs text-[var(--color-muted-foreground)]">
              Enter your Privacy PIN to continue
            </p>
          </div>
        </div>
        <Input
          type="password"
          inputMode="numeric"
          autoComplete="current-password"
          autoFocus
          placeholder="PIN"
          value={pin}
          onChange={(e) => setPin(e.target.value)}
          className="min-h-11"
          disabled={busy}
        />
        {error && <p className="text-sm text-[var(--color-destructive)]">{error}</p>}
        <Button type="submit" className="min-h-11 w-full" disabled={busy || pin.trim().length < 4}>
          {busy ? "Unlocking…" : "Unlock"}
        </Button>
      </form>
    </div>
  );
}
