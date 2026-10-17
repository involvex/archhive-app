import { create } from "zustand";

interface ShareState {
  /** Raw shared text waiting to be prefilled into Bulk import. Null when none. */
  pendingShare: string | null;
  setPendingShare: (text: string) => void;
  /** Take + clear. Returns the text or null when empty. */
  takePendingShare: () => string | null;
  clearPendingShare: () => void;
  /**
   * Shared cookie JSON waiting for the user to confirm which site it belongs to.
   * Kept separate from `pendingShare` so it never lands in the Bulk-import flow.
   */
  pendingCookieImport: string | null;
  setPendingCookieImport: (text: string) => void;
  clearPendingCookieImport: () => void;
}

export const useShareStore = create<ShareState>()((set, get) => ({
  pendingShare: null,
  setPendingShare: (text) => {
    const trimmed = text.trim();
    if (!trimmed) return;
    const prev = get().pendingShare;
    // Append follow-up shares instead of dropping the earlier one.
    set({ pendingShare: prev ? `${prev.trimEnd()}\n${trimmed}` : trimmed });
  },
  takePendingShare: () => {
    const current = get().pendingShare;
    if (current == null || !current.trim()) {
      if (current !== null) set({ pendingShare: null });
      return null;
    }
    set({ pendingShare: null });
    return current;
  },
  clearPendingShare: () => set({ pendingShare: null }),

  pendingCookieImport: null,
  setPendingCookieImport: (text) => {
    const trimmed = text.trim();
    if (!trimmed) return;
    // First share wins while the dialog is open; a second one replaces it so the
    // user can correct a misfire without reloading.
    set({ pendingCookieImport: trimmed });
  },
  clearPendingCookieImport: () => set({ pendingCookieImport: null }),
}));
