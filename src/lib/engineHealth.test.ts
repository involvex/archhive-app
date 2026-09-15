import { describe, it, expect } from "vitest";
import { enginePluginStatus } from "@/lib/engineHealth";

describe("enginePluginStatus (#48)", () => {
  it("is missing for null or all-empty payloads", () => {
    expect(enginePluginStatus(null)).toBe("missing");
    expect(enginePluginStatus({})).toBe("missing");
    expect(
      enginePluginStatus({ ytdlp_version: "  ", ffmpeg_version: "", ffprobe_version: "" }),
    ).toBe("missing");
  });

  it("is ready when engine and media tools all report", () => {
    expect(
      enginePluginStatus({
        ytdlp_version: "2026.01.01",
        ffmpeg_version: "6.1",
        ffprobe_version: "6.1",
      }),
    ).toBe("ready");
  });

  it("is degraded on partial payloads", () => {
    expect(enginePluginStatus({ ytdlp_version: "2026.01.01" })).toBe("degraded");
    expect(enginePluginStatus({ ffmpeg_version: "6.1", ffprobe_version: "6.1" })).toBe("degraded");
    expect(enginePluginStatus({ ytdlp_version: "2026.01.01", ffmpeg_version: "6.1" })).toBe(
      "degraded",
    );
  });
});
