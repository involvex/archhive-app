export type ShortcutCategory = "navigation" | "actions" | "view";

export interface Shortcut {
  id: string;
  label: string;
  keys: string;
  category: ShortcutCategory;
  action: () => void;
  enabled?: boolean;
}

const shortcuts: Shortcut[] = [];

export function registerShortcut(s: Shortcut): void {
  unregisterShortcut(s.id);
  shortcuts.push({ ...s, enabled: s.enabled ?? true });
}

export function unregisterShortcut(id: string): void {
  const idx = shortcuts.findIndex((s) => s.id === id);
  if (idx !== -1) shortcuts.splice(idx, 1);
}

export function getAllShortcuts(): Shortcut[] {
  return shortcuts.filter((s) => s.enabled !== false);
}

export function matchShortcut(e: KeyboardEvent): Shortcut | null {
  const target = e.target as HTMLElement;
  const inInput =
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target.isContentEditable;

  for (const s of getAllShortcuts()) {
    if (inInput) continue;
    if (parseKeys(s.keys).some((combo) => keyComboMatches(e, combo))) {
      return s;
    }
  }
  return null;
}

export function parseKeys(keys: string): string[][] {
  return keys.split("|").map((combo) => combo.split("+").map((k) => k.trim().toLowerCase()));
}

function keyComboMatches(e: KeyboardEvent, combo: string[]): boolean {
  const ctrl = combo.includes("ctrl") || combo.includes("mod");
  const shift = combo.includes("shift");
  const alt = combo.includes("alt") || combo.includes("opt");
  const meta = combo.includes("meta") || combo.includes("cmd");

  const keyPart = combo.find(
    (k) => !["ctrl", "shift", "alt", "meta", "mod", "opt", "cmd"].includes(k),
  );
  if (!keyPart) return false;

  // Q18: printable symbol keys (e.g. "?") require Shift on most layouts, so a
  // literal Shift mismatch must not block them. Compare the produced key first.
  const eventKey = e.key.toLowerCase();
  const needsShiftForSymbol =
    keyPart.length === 1 && !/[a-z0-9]/i.test(keyPart) && eventKey === keyPart;

  if (ctrl !== (e.ctrlKey || e.metaKey)) return false;
  if (!needsShiftForSymbol && shift !== e.shiftKey) return false;
  if (alt !== e.altKey) return false;
  if (meta !== e.metaKey) return false;

  if (keyPart === "space") return eventKey === " ";
  if (keyPart === "escape") return eventKey === "escape";
  if (keyPart === "enter") return eventKey === "enter";
  if (keyPart === "arrowleft") return eventKey === "arrowleft";
  if (keyPart === "arrowright") return eventKey === "arrowright";
  if (keyPart === "arrowup") return eventKey === "arrowup";
  if (keyPart === "arrowdown") return eventKey === "arrowdown";
  if (keyPart === "backspace") return eventKey === "backspace";
  if (keyPart === "delete") return eventKey === "delete";
  if (keyPart === "tab") return eventKey === "tab";

  return eventKey === keyPart;
}

export function formatShortcutKeys(keys: string): string {
  return keys
    .split("|")
    .map((combo) =>
      combo
        .split("+")
        .map((k) => k.trim())
        .map((k) => {
          if (k === "Ctrl" || k === "ctrl") return "Ctrl";
          if (k === "Shift" || k === "shift") return "Shift";
          if (k === "Alt" || k === "alt") return "Alt";
          if (k === "Meta" || k === "meta") return "Cmd";
          if (k === "Mod" || k === "mod") return "Ctrl";
          if (k.length === 1) return k.toUpperCase();
          return k.charAt(0).toUpperCase() + k.slice(1);
        })
        .join("+"),
    )
    .join(" / ");
}
