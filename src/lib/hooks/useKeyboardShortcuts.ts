import { useEffect } from "react";
import { matchShortcut } from "@/lib/shortcuts/registry";

export function useKeyboardShortcuts(): void {
  useEffect(() => {
    function handler(e: KeyboardEvent) {
      const target = e.target as HTMLElement;
      const inInput =
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        target.isContentEditable;
      if (inInput) return;

      const match = matchShortcut(e);
      if (match) {
        e.preventDefault();
        match.action();
      }
    }
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);
}
