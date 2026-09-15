import { describe, it, expect } from "vitest";
import {
  DOUBLE_TAP_MS,
  isDoubleTap,
  isScrub,
  formatSkipLabel,
  swipeSeekTarget,
  tapZone,
  type TouchPoint,
} from "@/lib/playerGestures";

const pt = (x: number, y: number, t: number): TouchPoint => ({ x, y, t });

describe("tapZone (#44)", () => {
  it("splits the width into thirds", () => {
    expect(tapZone(10, 300)).toBe("left");
    expect(tapZone(150, 300)).toBe("middle");
    expect(tapZone(290, 300)).toBe("right");
  });

  it("falls back to middle for degenerate widths", () => {
    expect(tapZone(5, 0)).toBe("middle");
  });
});

describe("isDoubleTap (#44)", () => {
  it("detects a quick second tap in place", () => {
    expect(isDoubleTap(pt(100, 100, 1000), pt(104, 102, 1000 + DOUBLE_TAP_MS))).toBe(true);
  });

  it("rejects slow or far-apart taps", () => {
    expect(isDoubleTap(null, pt(100, 100, 1000))).toBe(false);
    expect(isDoubleTap(pt(100, 100, 1000), pt(100, 100, 1000 + DOUBLE_TAP_MS + 1))).toBe(false);
    expect(isDoubleTap(pt(100, 100, 1000), pt(200, 100, 1100))).toBe(false);
  });
});

describe("isScrub (#44)", () => {
  it("needs enough horizontal travel", () => {
    expect(isScrub(30, 5)).toBe(true);
    expect(isScrub(10, 2)).toBe(false);
    expect(isScrub(30, 30)).toBe(false);
  });
});

describe("swipeSeekTarget (#44)", () => {
  it("maps full-width swipes to full-duration seeks, clamped", () => {
    expect(swipeSeekTarget(30, 150, 300, 120)).toBe(90);
    expect(swipeSeekTarget(110, 150, 300, 120)).toBe(120);
    expect(swipeSeekTarget(10, -150, 300, 120)).toBe(0);
  });

  it("holds position for unknown durations", () => {
    expect(swipeSeekTarget(30, 150, 300, NaN)).toBe(30);
    expect(swipeSeekTarget(30, 150, 0, 120)).toBe(30);
  });
});

describe("formatSkipLabel (#44)", () => {
  it("formats signed skip labels", () => {
    expect(formatSkipLabel(-10)).toBe("−10s");
    expect(formatSkipLabel(10)).toBe("+10s");
  });
});
