import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";

// Pretend to be the Android app, where HTML5 fullscreen is unavailable.
vi.mock("@/lib/runtime", () => ({ getAppRuntime: () => "mobile-tauri" }));
const invoke = vi.fn(async () => undefined);
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { HlsVideoPlayer } from "./HlsVideoPlayer";

const createElementStub = document.createElement;
beforeAll(() => {
  document.createElement = Document.prototype.createElement.bind(document);
});
afterAll(() => {
  document.createElement = createElementStub as typeof document.createElement;
});

describe("HlsVideoPlayer fullscreen in the Android app", () => {
  it("hides the dead native button and uses app fullscreen + immersive mode", async () => {
    const { container } = render(<HlsVideoPlayer src="https://example.com/a.mp4" />);
    const video = container.querySelector("video")!;
    expect(video.getAttribute("controlslist")).toBe("nofullscreen");

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Fullscreen" }));
    });
    const wrapper = video.parentElement!;
    expect(wrapper.className).toContain("fixed");
    expect(invoke).toHaveBeenCalledWith("set_player_fullscreen", {
      enabled: true,
      landscape: true,
    });

    // Android back button (popstate) leaves fullscreen and restores the bars.
    await act(async () => {
      window.dispatchEvent(new PopStateEvent("popstate"));
    });
    expect(wrapper.className).not.toContain("fixed");
    expect(invoke).toHaveBeenLastCalledWith("set_player_fullscreen", {
      enabled: false,
      landscape: false,
    });
  });
});
