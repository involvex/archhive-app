// @vitest-environment jsdom
import { afterAll, afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { SetupChecklistCard } from "./SetupChecklistCard";
import type { SetupChecklistItem } from "@/lib/setupChecklist";

vi.mock("@tanstack/react-router", () => ({
  Link: ({
    children,
    to,
    search,
    ...rest
  }: {
    children: React.ReactNode;
    to: string;
    search?: { tab?: string };
    className?: string;
  }) => (
    <a href={`${to}?tab=${search?.tab ?? ""}`} {...rest}>
      {children}
    </a>
  ),
}));

const createElementStub = document.createElement;
beforeAll(() => {
  document.createElement = Document.prototype.createElement.bind(document);
});
afterAll(() => {
  document.createElement = createElementStub as typeof document.createElement;
});

afterEach(() => cleanup());

const items: SetupChecklistItem[] = [
  {
    id: "library",
    title: "Confirm library path",
    description: "Where downloads live",
    tab: "library",
    done: true,
  },
  {
    id: "cookies",
    title: "Import site cookies",
    description: "For login sites",
    tab: "cookies",
    done: false,
  },
];

describe("SetupChecklistCard (R07)", () => {
  it("renders deep links and dismiss", () => {
    const onDismiss = vi.fn();
    render(<SetupChecklistCard items={items} onDismiss={onDismiss} />);

    expect(screen.getByText("Finish setup")).toBeTruthy();
    expect(screen.getByText("1/2 complete — open each item to adjust Settings")).toBeTruthy();
    const cookieLink = screen.getByRole("link", { name: /Import site cookies/i });
    expect(cookieLink.getAttribute("href")).toContain("tab=cookies");

    fireEvent.click(screen.getByRole("button", { name: /dismiss setup checklist/i }));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });
});
