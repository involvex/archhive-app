import { createRef } from "react";
import { render, waitFor } from "@testing-library/react";
import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { HlsVideoPlayer } from "./HlsVideoPlayer";

// vitest.setup stubs document.createElement for download helpers; restore real DOM
// creation so <video> mounts correctly in this suite.
const createElementStub = document.createElement;

beforeAll(() => {
  document.createElement = Document.prototype.createElement.bind(document);
});

afterAll(() => {
  document.createElement = createElementStub as typeof document.createElement;
});

describe("HlsVideoPlayer", () => {
  it("sets progressive src on the video element (merged ref)", async () => {
    const progressiveSrc = "https://example.com/library/clip.mp4";
    const videoRef = createRef<HTMLVideoElement>();

    render(
      <HlsVideoPlayer
        ref={videoRef}
        src={progressiveSrc}
        controls
        playsInline
        preload="metadata"
      />,
    );

    await waitFor(() => {
      expect(videoRef.current).toBeInstanceOf(HTMLVideoElement);
    });

    const video = videoRef.current!;
    expect(video.getAttribute("src")).toBe(progressiveSrc);
    // Property may be absolutized by the browser; still must include the path.
    expect(video.src).toContain("clip.mp4");
  });

  it("forwards the ref to the same element used for playback", async () => {
    const videoRef = createRef<HTMLVideoElement>();
    const { container } = render(<HlsVideoPlayer ref={videoRef} src="https://example.com/a.mp4" />);

    await waitFor(() => {
      expect(videoRef.current).toBeTruthy();
    });

    const fromDom = container.querySelector("video");
    expect(fromDom).toBe(videoRef.current);
  });
});
