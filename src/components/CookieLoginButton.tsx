import { useState } from "react";
import { LogIn } from "lucide-react";
import { api } from "@/lib/api/client";
import { isDesktopTauri, isTauri } from "@/lib/tauri";
import { Button } from "@/components/ui/button";

interface CookieLoginButtonProps {
  siteId: string;
  siteName?: string;
  /** Site address for per-domain entries (`host-<domain>`). */
  url?: string;
  /** Called after cookies were captured and stored. */
  onSaved?: (count: number) => void;
  className?: string;
}

/**
 * Opens the in-app login browser for a site and stores the captured cookies.
 *
 * Desktop opens a separate window: the user logs in there, then presses
 * "Save cookies" here (or simply closes the window). On Android a full-screen
 * browser with its own Save / Cancel bar is shown.
 */
export function CookieLoginButton({
  siteId,
  siteName,
  url,
  onSaved,
  className,
}: CookieLoginButtonProps) {
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("");
  const desktop = isDesktopTauri();

  if (!isTauri()) return null;

  async function start() {
    setBusy(true);
    setStatus(
      desktop
        ? `Log in to ${siteName ?? siteId} in the new window, then press Save cookies (or close it).`
        : "",
    );
    try {
      const result = await api.loginForCookies(siteId, url);
      setStatus(`Saved ${result.count} cookies for ${siteName ?? siteId}.`);
      onSaved?.(result.count);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setStatus(/cancelled/i.test(msg) ? "Login cancelled." : msg);
    } finally {
      setBusy(false);
    }
  }

  async function finish(save: boolean) {
    try {
      await api.finishCookieLogin(save);
    } catch (e) {
      setStatus(e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <div className={className}>
      <div className="flex flex-wrap gap-2">
        {!busy && (
          <Button onClick={() => void start()} disabled={!siteId}>
            <LogIn className="mr-2 h-4 w-4" />
            Log in with browser
          </Button>
        )}
        {busy && desktop && (
          <>
            <Button onClick={() => void finish(true)}>Save cookies</Button>
            <Button variant="outline" onClick={() => void finish(false)}>
              Cancel
            </Button>
          </>
        )}
        {busy && !desktop && (
          <Button disabled>
            <LogIn className="mr-2 h-4 w-4" />
            Login browser open…
          </Button>
        )}
      </div>
      {status && <p className="mt-2 text-xs text-[var(--color-muted-foreground)]">{status}</p>}
    </div>
  );
}
