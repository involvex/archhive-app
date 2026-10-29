import { create } from "zustand";
import { api } from "../api/client";
import { authenticateForVault } from "../biometrics";
import { setVaultMediaRoot, setVaultMediaUnlocked } from "../mediaUrl";
import type { VaultStatus, VaultScene } from "../types";

interface VaultState {
  status: VaultStatus | null;
  /** Vaulted rows, only loaded while the session is unlocked. */
  items: VaultScene[];
  loading: boolean;
  error: string | null;
  /** True while a biometric prompt is open, so the UI can show a spinner. */
  authenticating: boolean;
  /** Last reason the unlock was attempted without hardware backing. */
  degradedUnlock: boolean;

  refreshStatus: () => Promise<VaultStatus | null>;
  /** Biometric prompt, then open the backend session. */
  unlock: () => Promise<boolean>;
  lock: () => Promise<void>;
  loadItems: () => Promise<void>;
  vaultScene: (id: string) => Promise<void>;
  vaultScenes: (ids: string[]) => Promise<void>;
  restoreScene: (id: string) => Promise<void>;
  reset: () => void;
}

/** Mirror the backend session into the media-URL layer. */
function applyStatus(status: VaultStatus | null): void {
  setVaultMediaRoot(status?.vault_root);
  setVaultMediaUnlocked(status?.unlocked === true);
}

export const useVaultStore = create<VaultState>()((set, get) => ({
  status: null,
  items: [],
  loading: false,
  error: null,
  authenticating: false,
  degradedUnlock: false,

  refreshStatus: async () => {
    try {
      const status = await api.vaultStatus();
      applyStatus(status);
      set({ status, error: null });
      return status;
    } catch (e) {
      // A remote/browser runtime simply has no vault; don't shout about it.
      applyStatus(null);
      set({ status: null, error: e instanceof Error ? e.message : String(e) });
      return null;
    }
  },

  unlock: async () => {
    set({ authenticating: true, error: null });
    try {
      const outcome = await authenticateForVault();
      if (!outcome.ok) {
        set({ authenticating: false, error: outcome.message ?? null });
        return false;
      }
      const status = await api.vaultUnlock();
      applyStatus(status);
      set({ status, authenticating: false, degradedUnlock: outcome.degraded, error: null });
      await get().loadItems();
      return true;
    } catch (e) {
      set({
        authenticating: false,
        error: e instanceof Error ? e.message : String(e),
      });
      return false;
    }
  },

  lock: async () => {
    try {
      const status = await api.vaultLock();
      applyStatus(status);
      set({ status, items: [], degradedUnlock: false });
    } catch {
      // Locking must never fail loudly; a stale session is worse than a
      // silent no-op here, so drop the local copy either way.
      applyStatus(null);
      set({ items: [], status: null });
    }
  },

  loadItems: async () => {
    set({ loading: true });
    try {
      const items = await api.listVaultScenes();
      set({ items, loading: false, error: null });
    } catch (e) {
      set({
        items: [],
        loading: false,
        error: e instanceof Error ? e.message : String(e),
      });
    }
  },

  vaultScene: async (id: string) => {
    await api.vaultScene(id);
    await get().refreshStatus();
  },

  vaultScenes: async (ids: string[]) => {
    await api.vaultScenes(ids);
    await get().refreshStatus();
  },

  restoreScene: async (id: string) => {
    await api.unvaultScene(id);
    set((s) => ({ items: s.items.filter((item) => item.id !== id) }));
    await get().refreshStatus();
  },

  reset: () => {
    applyStatus(null);
    set({
      status: null,
      items: [],
      loading: false,
      error: null,
      authenticating: false,
      degradedUnlock: false,
    });
  },
}));
