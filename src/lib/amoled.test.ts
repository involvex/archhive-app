import { describe, it, expect, beforeEach } from "vitest";
import { isAmoled, setAmoled, initAmoled } from "@/lib/amoled";

beforeEach(() => {
  localStorage.clear();
  document.documentElement.classList.remove("amoled");
});

describe("amoled (Q44)", () => {
  it("is off by default", () => {
    expect(isAmoled()).toBe(false);
    expect(document.documentElement.classList.contains("amoled")).toBe(false);
  });

  it("setAmoled persists and toggles the class", () => {
    setAmoled(true);
    expect(isAmoled()).toBe(true);
    expect(document.documentElement.classList.contains("amoled")).toBe(true);

    setAmoled(false);
    expect(isAmoled()).toBe(false);
    expect(document.documentElement.classList.contains("amoled")).toBe(false);
  });

  it("initAmoled applies the persisted choice", () => {
    localStorage.setItem("archhive_amoled", "1");
    initAmoled();
    expect(document.documentElement.classList.contains("amoled")).toBe(true);
  });

  it("initAmoled leaves the class off when nothing is persisted", () => {
    initAmoled();
    expect(document.documentElement.classList.contains("amoled")).toBe(false);
  });
});
