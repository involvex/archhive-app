import { forwardRef, useCallback, useEffect, useRef, type Ref } from "react";

export const STREAM_URL_EXPIRED_ERROR =
  "Playback failed — the stream URL may have expired. Tap Retry for a fresh one.";

export interface HlsVideoPlayerProps {
  src: string;
  controls?: boolean;
  autoPlay?: boolean;
  playsInline?: boolean;
  preload?: "none" | "metadata" | "auto";
  className?: string;
  crossOrigin?: "anonymous" | "use-credentials";
  poster?: string;
  onError?: (error: MediaError | null) => void;
  onTimeUpdate?: () => void;
  onPause?: () => void;
  onPlay?: () => void;
  onEnded?: () => void;
}

export const HlsVideoPlayer = forwardRef<HTMLVideoElement, HlsVideoPlayerProps>(
  (
    {
      src,
      controls = true,
      autoPlay = false,
      playsInline = true,
      preload = "metadata",
      className,
      crossOrigin,
      poster,
      onError,
      onTimeUpdate,
      onPause,
      onPlay,
      onEnded,
    }: HlsVideoPlayerProps,
    ref: Ref<HTMLVideoElement>,
  ) => {
    const videoRef = useRef<HTMLVideoElement>(null);
    const hlsRef = useRef<import("hls.js").default | null>(null);

    const isHls = src.toLowerCase().endsWith(".m3u8");

    const loadHls = useCallback(async (video: HTMLVideoElement, url: string) => {
      if (hlsRef.current) {
        hlsRef.current.destroy();
        hlsRef.current = null;
      }

      try {
        const { default: Hls } = await import("hls.js");

        if (Hls.isSupported()) {
          const hls = new Hls({
            enableWorker: true,
            lowLatencyMode: true,
            capLevelToPlayerSize: true,
          });
          hlsRef.current = hls;
          hls.loadSource(url);
          hls.attachMedia(video);
          hls.on(Hls.Events.MANIFEST_PARSED, () => {
            void video.play().catch(() => {});
          });
        } else if (video.canPlayType("application/vnd.apple.mpegurl")) {
          video.src = url;
        } else {
          video.src = url;
        }
      } catch {
        video.src = url;
      }
    }, []);

    useEffect(() => {
      const video = videoRef.current;
      if (!video || !src) return;

      if (isHls) {
        void loadHls(video, src);
      } else {
        video.src = src;
      }

      return () => {
        if (hlsRef.current) {
          hlsRef.current.destroy();
          hlsRef.current = null;
        }
      };
    }, [src, isHls, loadHls]);

    const handleVideoError = (e: React.SyntheticEvent<HTMLVideoElement>) => {
      const mediaError = e.currentTarget.error;
      if (mediaError?.code === 4) {
        console.warn("[HlsVideoPlayer] stream URL expired", { src });
      }
      onError?.(mediaError);
    };

    return (
      <video
        ref={ref}
        key={src}
        controls={controls}
        autoPlay={autoPlay}
        playsInline={playsInline}
        preload={preload}
        poster={poster}
        crossOrigin={crossOrigin}
        className={className}
        onError={handleVideoError}
        onTimeUpdate={onTimeUpdate}
        onPause={onPause}
        onPlay={onPlay}
        onEnded={onEnded}
      >
        {isHls && <source src={src} type="application/x-mpegURL" />}
        <track kind="captions" />
      </video>
    );
  },
);
