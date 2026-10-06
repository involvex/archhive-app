import { create } from "zustand";

interface ShareState {
  /** Raw shared text waiting to be prefilled into Bulk import. Null when none. */
  pendingShare: string | null;
  setPendingShare: (text: string) => void;
  /** Take + clear. Returns the text or null when empty. */
  takePendingShare: () => string | null;
  clearPendingShare: () => void;
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
}));
