/**
 * User naming-template presets (R02). Device-local only — the active template
 * still lives in AppSettings.naming_template and needs Library Save.
 */

export interface NamingPreset {
  id: string;
  name: string;
  template: string;
}

export interface BuiltinNamingPreset {
  id: string;
  name: string;
  template: string;
  builtin: true;
}

const STORAGE_KEY = "archhive.namingPresets";

/** Built-in starters shown in the picker; load-only (not renamed/deleted). */
export const BUILTIN_NAMING_PRESETS: readonly BuiltinNamingPreset[] = [
  {
    id: "builtin:default",
    name: "Performer / title",
    template: "{performer}/{title}.{ext}",
    builtin: true,
  },
  {
    id: "builtin:site-performer",
    name: "Site / performer / title",
    template: "{site}/{performer}/{title}.{ext}",
    builtin: true,
  },
  {
    id: "builtin:date-title",
    name: "Date / title",
    template: "{date}/{title}.{ext}",
    builtin: true,
  },
] as const;

function newId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `preset-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

function isPreset(value: unknown): value is NamingPreset {
  if (typeof value !== "object" || value === null) return false;
  const p = value as NamingPreset;
  return (
    typeof p.id === "string" &&
    p.id.length > 0 &&
    typeof p.name === "string" &&
    typeof p.template === "string"
  );
}

export function loadNamingPresets(): NamingPreset[] {
  try {
    if (typeof localStorage === "undefined") return [];
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(isPreset).map((p) => ({
      id: p.id,
      name: p.name.trim() || "Untitled",
      template: p.template,
    }));
  } catch {
    return [];
  }
}

function persist(presets: NamingPreset[]): NamingPreset[] {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(presets));
    }
  } catch {
    /* private mode / quota — presets just won't persist */
  }
  return presets;
}

/**
 * Save the current template under `name`. Empty names are rejected.
 * If a user preset with the same name (case-insensitive) exists, overwrite it.
 */
export function saveNamingPreset(name: string, template: string): NamingPreset[] | null {
  const trimmed = name.trim();
  if (!trimmed) return null;
  const existing = loadNamingPresets();
  const lower = trimmed.toLowerCase();
  const idx = existing.findIndex((p) => p.name.toLowerCase() === lower);
  if (idx >= 0) {
    const next = [...existing];
    next[idx] = { ...next[idx], name: trimmed, template };
    return persist(next);
  }
  return persist([{ id: newId(), name: trimmed, template }, ...existing]);
}

/** Rename a user preset by id. Empty names rejected. Builtins are not in storage. */
export function renameNamingPreset(id: string, name: string): NamingPreset[] | null {
  const trimmed = name.trim();
  if (!trimmed) return null;
  const existing = loadNamingPresets();
  const idx = existing.findIndex((p) => p.id === id);
  if (idx < 0) return null;
  const lower = trimmed.toLowerCase();
  const clash = existing.findIndex((p, i) => i !== idx && p.name.toLowerCase() === lower);
  const next = [...existing];
  if (clash >= 0) {
    // Overwrite the other preset's name collision by removing it.
    next.splice(clash, 1);
    const adjusted = clash < idx ? idx - 1 : idx;
    next[adjusted] = { ...next[adjusted], name: trimmed };
  } else {
    next[idx] = { ...next[idx], name: trimmed };
  }
  return persist(next);
}

export function deleteNamingPreset(id: string): NamingPreset[] {
  return persist(loadNamingPresets().filter((p) => p.id !== id));
}

export function clearNamingPresets(): void {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.removeItem(STORAGE_KEY);
    }
  } catch {
    /* ignore */
  }
}

export function findPresetById(id: string): NamingPreset | BuiltinNamingPreset | undefined {
  const builtin = BUILTIN_NAMING_PRESETS.find((p) => p.id === id);
  if (builtin) return builtin;
  return loadNamingPresets().find((p) => p.id === id);
}
