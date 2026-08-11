import { registerShortcut } from "./registry";

let registered = false;

export function registerDefaultShortcuts(navigate: (path: string) => void): void {
  if (registered) return;
  registered = true;

  registerShortcut({
    id: "nav-home",
    label: "Go to Home",
    keys: "Ctrl+1",
    category: "navigation",
    action: () => navigate("/"),
  });
  registerShortcut({
    id: "nav-browse",
    label: "Go to Browse",
    keys: "Ctrl+2",
    category: "navigation",
    action: () => navigate("/browse"),
  });
  registerShortcut({
    id: "nav-library",
    label: "Go to Library",
    keys: "Ctrl+3",
    category: "navigation",
    action: () => navigate("/library"),
  });
  registerShortcut({
    id: "nav-live",
    label: "Go to Live",
    keys: "Ctrl+4",
    category: "navigation",
    action: () => navigate("/live"),
  });
  registerShortcut({
    id: "nav-downloads",
    label: "Go to Downloads",
    keys: "Ctrl+5",
    category: "navigation",
    action: () => navigate("/downloads"),
  });
  registerShortcut({
    id: "nav-settings",
    label: "Go to Settings",
    keys: "Ctrl+6",
    category: "navigation",
    action: () => navigate("/settings"),
  });
  registerShortcut({
    id: "cmd-palette",
    label: "Command Palette",
    keys: "Ctrl+K",
    category: "actions",
    action: () => {
      window.dispatchEvent(new CustomEvent("shortcut:cmd-palette"));
    },
  });
  registerShortcut({
    id: "new-download",
    label: "Paste URL to Download",
    keys: "Ctrl+N",
    category: "actions",
    action: () => navigate("/browse/by-url"),
  });
  registerShortcut({
    id: "shortcut-help",
    label: "Show Keyboard Shortcuts",
    keys: "?",
    category: "actions",
    action: () => {
      window.dispatchEvent(new CustomEvent("shortcut:help"));
    },
  });
}
