// @vitest-environment jsdom
import { afterAll, afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { SceneBulkEditBar } from "./SceneBulkEditBar";
import { api } from "@/lib/api/client";

// goober (used by react-hot-toast) injects <style> tags via document.createElement,
// which vitest.setup replaces with a download-anchor stub for other suites.
vi.mock("react-hot-toast", () => ({
  toast: Object.assign(vi.fn(), { success: vi.fn(), error: vi.fn() }),
}));

const createElementStub = document.createElement;
beforeAll(() => {
  document.createElement = Document.prototype.createElement.bind(document);
});
afterAll(() => {
  document.createElement = createElementStub as typeof document.createElement;
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("SceneBulkEditBar — suggest tags (R03)", () => {
  it("unions suggestions across the selection into one batch update", async () => {
    const suggest = vi
      .spyOn(api, "suggestSceneTags")
      .mockImplementation(async (id) =>
        id === "s1"
          ? { performers: ["Alice"], tags: ["Tag A"] }
          : { performers: ["Bob"], tags: ["tag a", "Tag B"] },
      );
    const batch = vi.spyOn(api, "batchUpdateScenes").mockResolvedValue({ updated: 2 });
    const onApplied = vi.fn();
    const onClear = vi.fn();

    render(<SceneBulkEditBar selectedIds={["s1", "s2"]} onClear={onClear} onApplied={onApplied} />);
    fireEvent.click(screen.getByRole("button", { name: /suggest tags/i }));

    await waitFor(() => expect(batch).toHaveBeenCalledTimes(1));
    expect(suggest).toHaveBeenCalledTimes(2);
    expect(batch.mock.calls[0][0]).toMatchObject({
      scene_ids: ["s1", "s2"],
      tags_add: ["Tag A", "Tag B"],
      performers_add: ["Alice", "Bob"],
    });
    expect(onApplied).toHaveBeenCalledTimes(1);
    expect(onClear).toHaveBeenCalledTimes(1);
  });

  it("does not call the batch API when no suggestions are found", async () => {
    vi.spyOn(api, "suggestSceneTags").mockResolvedValue({ performers: [], tags: [] });
    const batch = vi.spyOn(api, "batchUpdateScenes").mockResolvedValue({ updated: 0 });

    render(<SceneBulkEditBar selectedIds={["s1"]} onClear={vi.fn()} onApplied={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: /suggest tags/i }));

    await waitFor(() => expect(batch).not.toHaveBeenCalled());
  });
});
