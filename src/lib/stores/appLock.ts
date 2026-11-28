import { create } from "zustand";
import { hasVaultPinConfigured, shouldEngageAppLock } from "@/lib/appLock";
import { useSettingsStore } from "@/lib/stores/settings";

interface AppLockState {
  locked: boolean;
  /** Epoch ms when the document last became hidden; null until first hide. */
  hiddenAt: number | null;
  /** After first hydrate, cold-start lock has been evaluated. */
  coldStartChecked: boolean;

  lock: () => void;
  unlock: () => void;
  markHidden: (at?: number) => void;
  /** Call on visibility/focus return — may engage the lock. */
  checkResume: (now?: number) => void;
  /** Call once settings are hydrated (cold start). */
  checkColdStart: () => void;
}

function lockEligible(): { enabled: boolean; pinConfigured: boolean; timeoutMins: number } {
  const { settings } = useSettingsStore.getState();
  return {
    enabled: settings.app_lock_enabled === true,
    pinConfigured: hasVaultPinConfigured(settings.vault_pin_hash),
    timeoutMins: settings.app_lock_timeout_mins ?? 5,
  };
}

export const useAppLockStore = create<AppLockState>()((set, get) => ({
  locked: false,
  hiddenAt: null,
  coldStartChecked: false,

  lock: () => set({ locked: true }),
  unlock: () => set({ locked: false, hiddenAt: null }),

  markHidden: (at = Date.now()) => set({ hiddenAt: at }),

  checkResume: (now = Date.now()) => {
    const { enabled, pinConfigured, timeoutMins } = lockEligible();
    const { hiddenAt } = get();
    if (
      shouldEngageAppLock({
        enabled,
        pinConfigured,
        hiddenAt: hiddenAt ?? now,
        now,
        timeoutMins,
      })
    ) {
      set({ locked: true });
    }
  },

  checkColdStart: () => {
    if (get().coldStartChecked) return;
    const { enabled, pinConfigured, timeoutMins } = lockEligible();
    const engage = shouldEngageAppLock({
      enabled,
      pinConfigured,
      hiddenAt: null,
      now: Date.now(),
      timeoutMins,
    });
    set({ coldStartChecked: true, locked: engage });
  },
}));
