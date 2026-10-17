// @vitest-environment jsdom
import { afterAll, afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { SceneContextMenu } from "./SceneContextMenu";
import type { Scene } from "@/lib/types";

const scene = {
  id: "s1",
  title: "Test clip",
  path: "/lib/test.mp4",
  performers: [],
  tags: [],
} as unknown as Scene;

function setUserAgent(ua: string) {
  Object.defineProperty(window.navigator, "userAgent", { value: ua, configurable: true });
}

function renderMenu() {
  return render(
    <SceneContextMenu
      menu={{ scene, x: 900, y: 690 }}
      onClose={vi.fn()}
      onEdit={vi.fn()}
      onDetails={vi.fn()}
      onPlay={vi.fn()}
      onOpenExplorer={vi.fn()}
      onOpenDefault={vi.fn()}
    />,
  );
}

// vitest.setup stubs document.createElement for download helpers; restore the
// real DOM factory so the menu mounts (same as HlsVideoPlayer.test.tsx).
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

describe("SceneContextMenu", () => {
  it("renders a bottom sheet on phones without the desktop-only Explorer item", () => {
    setUserAgent("Mozilla/5.0 (Linux; Android 14; Pixel 8) Mobile");
    renderMenu();
    const menu = screen.getByRole("menu");
    expect(menu.className).toContain("rounded-t-2xl");
    expect(menu.className).toContain("safe-area-inset-bottom");
    expect(menu.style.top).toBe("");
    expect(screen.queryByText("Open in Explorer")).toBeNull();
    expect(screen.getByText("Play")).toBeTruthy();
  });

  it("keeps the desktop popup inside the window near the bottom-right edge", () => {
    setUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)");
    Object.defineProperty(window, "innerWidth", { value: 1000, configurable: true });
    Object.defineProperty(window, "innerHeight", { value: 700, configurable: true });
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
      width: 200,
      height: 240,
    } as DOMRect);
    renderMenu();
    const menu = screen.getByRole("menu");
    expect(parseInt(menu.style.left, 10)).toBeLessThanOrEqual(1000 - 200 - 8);
    expect(parseInt(menu.style.top, 10) + 240).toBeLessThanOrEqual(700);
    expect(menu.style.visibility).toBe("visible");
  });
});
