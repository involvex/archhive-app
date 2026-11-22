import { describe, expect, it } from "vitest";
import { analyseNamingTemplate } from "@/lib/namingTemplate";

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
