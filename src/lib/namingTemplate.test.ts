import { describe, expect, it } from "vitest";
import { analyseNamingTemplate, insertNamingToken } from "@/lib/namingTemplate";

describe("analyseNamingTemplate", () => {
  it("renders a sample path for the default template", () => {
    const result = analyseNamingTemplate("{performer}/{title}.{ext}");
    expect(result.preview).toBe("JaneDoe/Sample Title.mp4");
    expect(result.unknownTokens).toEqual([]);
    expect(result.hasIllegalChars).toBe(false);
    expect(result.fallsBackToDefault).toBe(false);
    expect(result.ytdlpTemplate).toBe("%(uploader)s/%(title)s.%(ext)s");
  });

  it("flags unknown tokens and falls back when none are known", () => {
    const result = analyseNamingTemplate("{studio}/{foo}.{ext}");
    expect(result.unknownTokens).toContain("studio");
    expect(result.unknownTokens).toContain("foo");
    // {ext} is known, so it does not fall back entirely.
    expect(result.fallsBackToDefault).toBe(false);
    expect(result.ytdlpTemplate).toContain("%(ext)s");
  });

  it("falls back when the template has no known tokens", () => {
    const result = analyseNamingTemplate("{studio}/{foo}");
    expect(result.fallsBackToDefault).toBe(true);
    expect(result.preview).toBe("JaneDoe/Sample Title.mp4");
  });

  it("detects illegal Windows path characters", () => {
    expect(analyseNamingTemplate("{title}: {id}.{ext}").hasIllegalChars).toBe(true);
  });
});

describe("insertNamingToken", () => {
  it("inserts at the caret", () => {
    const result = insertNamingToken("{performer}/", "title", 12);
    expect(result.value).toBe("{performer}/{title}");
    expect(result.caret).toBe("{performer}/{title}".length);
  });

  it("replaces a selection", () => {
    // Replace the whole `{foo}` span (indices 0..5).
    const result = insertNamingToken("{foo}/{title}.{ext}", "performer", 0, 5);
    expect(result.value).toBe("{performer}/{title}.{ext}");
    expect(result.caret).toBe("{performer}".length);
  });

  it("clamps out-of-range indices", () => {
    const result = insertNamingToken("a", "ext", 99, 200);
    expect(result.value).toBe("a{ext}");
    expect(result.caret).toBe("a{ext}".length);
  });

  it("handles reversed start/end", () => {
    const result = insertNamingToken("ab", "id", 2, 0);
    expect(result.value).toBe("{id}");
    expect(result.caret).toBe(4);
  });
});
