import { beforeEach, describe, expect, it } from "vitest";
import {
  BUILTIN_NAMING_PRESETS,
  clearNamingPresets,
  deleteNamingPreset,
  findPresetById,
  loadNamingPresets,
  renameNamingPreset,
  saveNamingPreset,
} from "@/lib/namingPresets";

describe("namingPresets (R02)", () => {
  beforeEach(() => {
    clearNamingPresets();
    localStorage.clear();
  });

  it("starts empty", () => {
    expect(loadNamingPresets()).toEqual([]);
  });

  it("saves a named preset", () => {
    const next = saveNamingPreset("Flat", "{title}.{ext}");
    expect(next).toHaveLength(1);
    expect(next![0]).toMatchObject({ name: "Flat", template: "{title}.{ext}" });
    expect(loadNamingPresets()).toHaveLength(1);
  });

  it("rejects blank names", () => {
    expect(saveNamingPreset("   ", "{title}.{ext}")).toBeNull();
    expect(loadNamingPresets()).toEqual([]);
  });

  it("overwrites by name case-insensitively", () => {
    saveNamingPreset("Flat", "{title}.{ext}");
    const next = saveNamingPreset("flat", "{performer}/{title}.{ext}");
    expect(next).toHaveLength(1);
    expect(next![0].template).toBe("{performer}/{title}.{ext}");
    expect(next![0].name).toBe("flat");
  });

  it("renames a user preset", () => {
    const saved = saveNamingPreset("Old", "{title}.{ext}")!;
    const id = saved[0].id;
    const next = renameNamingPreset(id, "New");
    expect(next).toHaveLength(1);
    expect(next![0].name).toBe("New");
  });

  it("rejects blank rename", () => {
    const saved = saveNamingPreset("Old", "{title}.{ext}")!;
    expect(renameNamingPreset(saved[0].id, "  ")).toBeNull();
  });

  it("deletes a user preset", () => {
    const saved = saveNamingPreset("Gone", "{title}.{ext}")!;
    expect(deleteNamingPreset(saved[0].id)).toEqual([]);
  });

  it("falls back to empty on corrupt storage", () => {
    localStorage.setItem("archhive.namingPresets", "not-json");
    expect(loadNamingPresets()).toEqual([]);
  });

  it("finds builtins and user presets by id", () => {
    expect(findPresetById(BUILTIN_NAMING_PRESETS[0].id)?.template).toBe(
      "{performer}/{title}.{ext}",
    );
    const saved = saveNamingPreset("Mine", "{date}/{title}.{ext}")!;
    expect(findPresetById(saved[0].id)?.template).toBe("{date}/{title}.{ext}");
    expect(findPresetById("missing")).toBeUndefined();
  });
});
