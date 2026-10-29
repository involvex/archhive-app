import { Link, createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import { vaultThumbUrl } from "@/lib/mediaUrl";
import { useVaultStore } from "@/lib/stores/vault";
import { useSettingsStore } from "@/lib/stores/settings";
import { getAppRuntime } from "@/lib/runtime";
import type { Scene, VaultScene } from "@/lib/types";
import { ScenePlayerDialog } from "@/components/ScenePlayerDialog";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Lock, LockOpen, RefreshCw, ShieldCheck, Undo2 } from "lucide-react";

export const Route = createFileRoute("/library/vault")({
  component: VaultPage,
});

function formatBytes(bytes?: number): string {
  if (!bytes || bytes <= 0) return "";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  let size = bytes;
  while (size >= 1024 && i < units.length - 1) {
    size /= 1024;
    i++;
  }
  return `${size.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

function VaultPage() {
  const settings = useSettingsStore((s) => s.settings);
  const status = useVaultStore((s) => s.status);
  const items = useVaultStore((s) => s.items);
  const loading = useVaultStore((s) => s.loading);
  const authenticating = useVaultStore((s) => s.authenticating);
  const error = useVaultStore((s) => s.error);
  const degradedUnlock = useVaultStore((s) => s.degradedUnlock);
  const refreshStatus = useVaultStore((s) => s.refreshStatus);
  const unlock = useVaultStore((s) => s.unlock);
  const lock = useVaultStore((s) => s.lock);
  const loadItems = useVaultStore((s) => s.loadItems);
  const restoreScene = useVaultStore((s) => s.restoreScene);

  const [playerScene, setPlayerScene] = useState<Scene | null>(null);
  const [playingId, setPlayingId] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [localError, setLocalError] = useState<string | null>(null);

  useEffect(() => {
    void refreshStatus();
  }, [refreshStatus]);

  useEffect(() => {
    if (status?.unlocked) {
      void loadItems();
    }
  }, [status?.unlocked, loadItems]);

  const play = useCallback(async (item: VaultScene) => {
    setPlayingId(item.id);
    setLocalError(null);
    try {
      const scene = await api.getScene(item.id);
      setPlayerScene(scene);
    } catch (e) {
      setLocalError(e instanceof Error ? e.message : String(e));
    } finally {
      setPlayingId(null);
    }
  }, []);

  async function restore(item: VaultScene) {
    setBusyId(item.id);
    setLocalError(null);
    try {
      await restoreScene(item.id);
    } catch (e) {
      setLocalError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusyId(null);
    }
  }

  const vaultEnabled = settings.vault_enabled === true;
  const unlocked = status?.unlocked === true;
  const isMobile = getAppRuntime() === "mobile-tauri";

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between gap-3">
        <div>
          <h2 className="text-2xl font-bold">Vault</h2>
          <p className="text-sm text-[var(--color-muted-foreground)]">
            Locked-away scenes. Files are moved out of the library folder while vaulted, so other
            apps and gallery scanners don&apos;t see them.
          </p>
        </div>
        <div className="flex shrink-0 gap-2">
          {unlocked ? (
            <Button variant="outline" size="sm" className="gap-1.5" onClick={() => void lock()}>
              <Lock className="h-3.5 w-3.5" />
              Lock
            </Button>
          ) : (
            <Button
              size="sm"
              className="gap-1.5"
              disabled={authenticating || !vaultEnabled}
              onClick={() => void unlock()}
            >
              <LockOpen className="h-3.5 w-3.5" />
              {authenticating ? "Checking…" : "Unlock"}
            </Button>
          )}
          <Button
            variant="outline"
            size="sm"
            className="gap-1.5"
            onClick={() => void refreshStatus()}
          >
            <RefreshCw className="h-3.5 w-3.5" />
            Refresh
          </Button>
        </div>
      </div>

      {(error || localError) && <p className="text-sm text-yellow-400">{localError ?? error}</p>}
      {degradedUnlock && (
        <p className="text-xs text-yellow-400">
          No biometrics available on this device — the vault was opened with a plain confirmation.
        </p>
      )}

      {!vaultEnabled ? (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">The vault is off</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3 text-sm">
            {status && status.item_count > 0 && (
              <p className="rounded-md border border-yellow-500/40 bg-yellow-500/10 p-2 text-yellow-300">
                {status.item_count} scene(s) are still in the vault, and their files stay hidden
                while the feature is off. Re-enable it to unlock and restore them.
              </p>
            )}
            <p className="text-[var(--color-muted-foreground)]">
              Enable it in <strong>Settings → Privacy</strong>, then use &ldquo;Move to vault&rdquo;
              on any scene.
            </p>
            <Link to="/settings" className="inline-block text-[var(--color-primary)] underline">
              Open settings
            </Link>
          </CardContent>
        </Card>
      ) : !unlocked ? (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-base">
              <Lock className="h-4 w-4" />
              Vault locked
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3 text-sm">
            <p className="text-[var(--color-muted-foreground)]">
              {status
                ? `${status.item_count} scene(s) are hidden.`
                : "Unlock to see what's inside."}{" "}
              {isMobile
                ? "Unlock with your fingerprint, face or device PIN."
                : "Desktop builds confirm the unlock with a dialog."}
            </p>
            <Button className="gap-1.5" disabled={authenticating} onClick={() => void unlock()}>
              <LockOpen className="h-4 w-4" />
              {authenticating ? "Waiting for confirmation…" : "Unlock vault"}
            </Button>
          </CardContent>
        </Card>
      ) : (
        <>
          <Card>
            <CardContent className="flex flex-wrap items-center gap-x-6 gap-y-2 p-4 text-xs text-[var(--color-muted-foreground)]">
              <span className="flex items-center gap-1.5 text-green-400">
                <ShieldCheck className="h-3.5 w-3.5" /> Unlocked for this session
              </span>
              <span>{items.length} item(s)</span>
              {status?.auto_lock_mins ? (
                <span>Auto-locks after {status.auto_lock_mins} min idle</span>
              ) : (
                <span>Auto-lock off</span>
              )}
              <span>LAN access: {status?.lan_exposed ? "exposed" : "hidden from the network"}</span>
            </CardContent>
          </Card>

          {loading ? (
            <p className="text-sm text-[var(--color-muted-foreground)]">Loading…</p>
          ) : items.length === 0 ? (
            <Card>
              <CardContent className="p-6 text-center text-sm text-[var(--color-muted-foreground)]">
                Nothing in the vault yet. Open a scene&apos;s context menu →{" "}
                <strong>Move to vault</strong>.
              </CardContent>
            </Card>
          ) : (
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
              {items.map((item) => {
                const thumb = vaultThumbUrl(item);
                return (
                  <Card key={item.id} className="overflow-hidden">
                    <button
                      type="button"
                      className="relative block aspect-video w-full bg-[var(--color-secondary)]"
                      onClick={() => void play(item)}
                      disabled={playingId === item.id}
                    >
                      {thumb ? (
                        <img
                          src={thumb}
                          alt={item.title}
                          loading="lazy"
                          className="h-full w-full object-cover"
                        />
                      ) : (
                        <span className="flex h-full items-center justify-center text-xs text-[var(--color-muted-foreground)]">
                          No preview
                        </span>
                      )}
                      {playingId === item.id && (
                        <span className="absolute inset-0 flex items-center justify-center bg-black/50 text-xs text-white">
                          Opening…
                        </span>
                      )}
                    </button>
                    <CardContent className="space-y-2 p-3">
                      <p className="line-clamp-2 text-sm font-medium" title={item.title}>
                        {item.title}
                      </p>
                      <div className="flex items-center justify-between gap-2">
                        <span className="text-xs text-[var(--color-muted-foreground)]">
                          {formatBytes(item.file_size)}
                        </span>
                        <Button
                          variant="outline"
                          size="sm"
                          className="gap-1"
                          disabled={busyId === item.id}
                          onClick={() => void restore(item)}
                        >
                          <Undo2 className="h-3 w-3" />
                          {busyId === item.id ? "Moving…" : "Restore"}
                        </Button>
                      </div>
                    </CardContent>
                  </Card>
                );
              })}
            </div>
          )}
        </>
      )}

      <ScenePlayerDialog
        scene={playerScene}
        scenes={playerScene ? [playerScene] : []}
        currentIndex={0}
        open={playerScene !== null}
        onClose={() => setPlayerScene(null)}
        onNavigate={(s) => setPlayerScene(s)}
      />
    </div>
  );
}
