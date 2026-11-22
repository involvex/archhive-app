import { describe, expect, it } from "vitest";
import { parseNoteTimestamps, splitNotesWithTimestamps } from "@/lib/noteTimestamps";

describe("parseNoteTimestamps", () => {
  it("parses mm:ss markers", () => {
    expect(parseNoteTimestamps("intro [0:12] then [1:05] end")).toEqual([
      { index: 6, raw: "[0:12]", seconds: 12, label: "0:12" },
      { index: 18, raw: "[1:05]", seconds: 65, label: "1:05" },
    ]);
  });

  it("parses h:mm:ss markers", () => {
    expect(parseNoteTimestamps("jump [1:02:03]")).toEqual([
      { index: 5, raw: "[1:02:03]", seconds: 3723, label: "1:02:03" },
    ]);
  });

  it("returns empty for missing or plain notes", () => {
    expect(parseNoteTimestamps(undefined)).toEqual([]);
    expect(parseNoteTimestamps("no stamps here")).toEqual([]);
  });
});

describe("splitNotesWithTimestamps", () => {
  it("interleaves text and timestamps", () => {
    expect(splitNotesWithTimestamps("before [0:45] after")).toEqual([
      { type: "text", text: "before " },
      { type: "timestamp", text: "[0:45]", seconds: 45, label: "0:45" },
      { type: "text", text: " after" },
    ]);
  });
});
