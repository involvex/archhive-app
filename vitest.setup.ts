import { cleanup } from "@testing-library/react";
import { afterEach, vi, beforeAll } from "vitest";
import "@testing-library/jest-dom";

beforeAll(() => {
  // Ensure document.body exists for happy-dom
  if (!document.body) {
    const body = document.createElement("body");
    document.documentElement.appendChild(body);
  }
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

// Mock Tauri APIs
vi.mock("@tauri-apps/api", () => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  emit: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
  save: vi.fn(),
}));

Object.defineProperty(window, "confirm", {
  value: vi.fn(() => true),
  writable: true,
});

Object.defineProperty(document, "createElement", {
  value: vi.fn(() => ({
    href: "",
    download: "",
    click: vi.fn(),
  })),
  writable: true,
});

Object.defineProperty(URL, "createObjectURL", {
  value: vi.fn(() => "blob:mock"),
  writable: true,
});

Object.defineProperty(URL, "revokeObjectURL", {
  value: vi.fn(),
  writable: true,
});

global.Blob = class Blob {
  constructor(
    public content: string[],
    public options: { type: string },
  ) {}
} as unknown as typeof Blob;
