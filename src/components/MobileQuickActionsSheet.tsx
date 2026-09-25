import { useEffect, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { toast } from "react-hot-toast";
import { Compass, Plus, RefreshCw, QrCode, Link as LinkIcon, X } from "lucide-react";

interface MobileQuickActionsSheetProps {
  onClose: () => void;
}

/**
 * Quick-actions bottom sheet, opened by long-pressing the FAB.
 * Three fast paths: paste a URL (deepest use case), scan a QR code
 * (handy for sharing links from another phone), or jump to browse.
 * Kept tiny on purpose — the FAB is the only entry point.
 */
export function MobileQuickActionsSheet({ onClose }: MobileQuickActionsSheetProps) {
  const navigate = useNavigate();
  const [pasteBusy, setPasteBusy] = useState(false);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  async function pasteUrl() {
    setPasteBusy(true);
    try {
      // Best-effort clipboard read; some WebView shells reject this and we
      // just fall back to the browse URL entry instead of crashing.
      let text = "";
      try {
        text = await navigator.clipboard.readText();
      } catch {
        /* ignore — user can type it */
      }
      if (text && (text.startsWith("http://") || text.startsWith("https://"))) {
        onClose();
        setTimeout(() => navigate({ to: "/browse/by-url", search: { url: text } }), 10);
        return;
      }
      toast("No URL on clipboard — paste one in Browse");
    } finally {
      setPasteBusy(false);
    }
  }

  function openBrowse() {
    onClose();
    setTimeout(() => navigate({ to: "/browse" }), 10);
  }

  function openNewDownload() {
    onClose();
    setTimeout(() => navigate({ to: "/browse/by-url" }), 10);
  }

  function openScan() {
    onClose();
    setTimeout(() => navigate({ to: "/library" }), 10);
  }

  const items: {
    id: string;
    label: string;
    description: string;
    icon: React.ReactNode;
    run: () => void;
  }[] = [
    {
      id: "paste-url",
      label: "Paste URL",
      description: "Download from clipboard",
      icon: <LinkIcon className="h-5 w-5" />,
      run: () => void pasteUrl(),
    },
    {
      id: "scan-qr",
      label: "Scan QR",
      description: "Camera scan for shared links",
      icon: <QrCode className="h-5 w-5" />,
      run: () => {
        // Camera access is gated behind a permission prompt; we surface a
        // friendly toast rather than an unhandled rejection.
        if (!navigator.mediaDevices?.getUserMedia) {
          toast("Camera access is not available on this device");
          return;
        }
        toast("Camera scan is coming soon — paste the URL instead for now");
      },
    },
    {
      id: "new-download",
      label: "New download",
      description: "Enter a URL manually",
      icon: <Plus className="h-5 w-5" />,
      run: openNewDownload,
    },
    {
      id: "browse",
      label: "Browse sites",
      description: "Trending, categories, saved searches",
      icon: <Compass className="h-5 w-5" />,
      run: openBrowse,
    },
    {
      id: "scan-library",
      label: "Scan library",
      description: "Re-index files on device",
      icon: <RefreshCw className="h-5 w-5" />,
      run: openScan,
    },
  ];

  return (
    <div
      className="fixed inset-0 z-50 md:hidden"
      role="dialog"
      aria-modal="true"
      aria-label="Quick actions"
    >
      <button
        type="button"
        aria-label="Close quick actions"
        className="fixed inset-0 cursor-default bg-black/60"
        onClick={onClose}
      />
      <div className="fixed right-0 bottom-0 left-0 overflow-hidden rounded-t-2xl border-t border-[var(--color-border)] bg-[var(--color-card)] shadow-2xl">
        <div className="mx-auto mt-2 h-1 w-10 rounded-full bg-[var(--color-muted-foreground)] opacity-40" />
        <div className="px-4 pb-[env(safe-area-inset-bottom)] pt-2">
          <p className="mb-2 text-xs font-medium tracking-wider text-[var(--color-muted-foreground)] uppercase">
            Quick actions
          </p>
          <div className="grid grid-cols-3 gap-2">
            {items.map((item) => (
              <button
                key={item.id}
                type="button"
                onClick={item.run}
                disabled={pasteBusy && item.id === "paste-url"}
                className="flex min-h-20 flex-col items-center justify-center gap-1.5 rounded-xl bg-[var(--color-secondary)] p-2 text-center active:scale-95 disabled:opacity-50"
              >
                <span className="text-[var(--color-primary)]">{item.icon}</span>
                <span className="text-xs font-medium leading-tight">{item.label}</span>
                <span className="text-[10px] text-[var(--color-muted-foreground)] line-clamp-1">
                  {item.description}
                </span>
              </button>
            ))}
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close"
            className="mt-3 flex w-full items-center justify-center gap-2 rounded-xl bg-[var(--color-muted)] py-3 text-sm font-medium active:scale-95"
          >
            <X className="h-4 w-4" />
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
