import { describe, expect, it } from "vitest";
import type { Scene } from "@/lib/types";
import {
  formatDuration,
  formatFileSize,
  formatResolution,
  pickBestDuplicateScene,
} from "@/lib/duplicates";

function scene(partial: Partial<Scene> & { id: string }): Scene {
  return {
    title: partial.title ?? partial.id,
    performers: [],
    tags: [],
    ...partial,
  };
}

describe("pickBestDuplicateScene", () => {
  it("prefers higher resolution, then larger file", () => {
    const best = pickBestDuplicateScene([
      scene({ id: "a", width: 1280, height: 720, file_size: 900 }),
      scene({ id: "b", width: 1920, height: 1080, file_size: 100 }),
      scene({ id: "c", width: 1920, height: 1080, file_size: 500 }),
    ]);
    expect(best?.id).toBe("c");
  });

  it("returns null for an empty list", () => {
    expect(pickBestDuplicateScene([])).toBeNull();
  });
});

describe("format helpers", () => {
  it("formats size, duration, and resolution", () => {
    expect(formatFileSize(1536)).toBe("2 KB");
    expect(formatDuration(125)).toBe("2:05");
    expect(formatResolution(1920, 1080)).toBe("1920×1080");
    expect(formatResolution(undefined, 1080)).toBe("—");
  });
});
