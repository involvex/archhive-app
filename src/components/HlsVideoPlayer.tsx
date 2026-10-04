import { forwardRef, useCallback, useEffect, useRef, useState, type Ref } from "react";
import { PictureInPicture2, VolumeX } from "lucide-react";

export const STREAM_URL_EXPIRED_ERROR =
  "Playback failed — the stream URL may have expired. Tap Retry for a fresh one.";

export interface HlsVideoPlayerProps {
  src: string;
  /**
   * Optional separate audio-only HLS URL (Chaturbate/Stripchat live serve
   * split audio/video chunklists). Played in a hidden `<audio>` element kept
   * in sync with the video element (play/pause/volume), because browsers
   * can't mux two HLS URLs in one `<video>` tag.
   */
  audioSrc?: string;
  controls?: boolean;
  autoPlay?: boolean;
  playsInline?: boolean;
  preload?: "none" | "metadata" | "auto";
  className?: string;
  style?: React.CSSProperties;
  crossOrigin?: "anonymous" | "use-credentials";
  poster?: string;
  /** Show an explicit Picture-in-Picture control (HTML PiP API). */
  showPip?: boolean;
  onError?: (error: MediaError | null) => void;
  onTimeUpdate?: () => void;
  onPause?: () => void;
  onPlay?: () => void;
  onEnded?: () => void;
}

function assignRef<T>(ref: Ref<T> | undefined, value: T | null) {
  if (!ref) return;
  if (typeof ref === "function") {
    ref(value);
  } else {
    ref.current = value;
  }
}

export const HlsVideoPlayer = forwardRef<HTMLVideoElement, HlsVideoPlayerProps>(
  (
    {
      src,
      audioSrc,
      controls = true,
      autoPlay = false,
      playsInline = true,
      preload = "metadata",
      className,
      style,
      crossOrigin,
      poster,
      showPip = true,
      onError,
      onTimeUpdate,
      onPause,
      onPlay,
      onEnded,
    }: HlsVideoPlayerProps,
    ref: Ref<HTMLVideoElement>,
  ) => {
    const videoRef = useRef<HTMLVideoElement | null>(null);
    const audioRef = useRef<HTMLAudioElement | null>(null);
    const hlsRef = useRef<import("hls.js").default | null>(null);
    const audioHlsRef = useRef<import("hls.js").default | null>(null);
    const lastSyncRef = useRef(0);
    const [needsUnmute, setNeedsUnmute] = useState(false);
    const [pipAvailable, setPipAvailable] = useState(false);

    const setVideoRef = useCallback(
      (node: HTMLVideoElement | null) => {
        videoRef.current = node;
        assignRef(ref, node);
        setPipAvailable(
          Boolean(
            node &&
            typeof node.requestPictureInPicture === "function" &&
            document.pictureInPictureEnabled,
          ),
        );
      },
      [ref],
    );

    const isHls = (() => {
      const lower = src.toLowerCase();
      // Loopback proxy URLs put the real path in ?url=...m3u8..., so pathname
      // is /api/media/proxy — check the full string, not just pathname.
      if (lower.includes(".m3u8")) return true;
      try {
        const path = new URL(src).pathname.toLowerCase();
        return path.endsWith(".m3u8") || path.includes(".m3u8/");
      } catch {
        return false;
      }
    })();

    const tryPlayWithAudio = useCallback(
      (video: HTMLVideoElement) => {
        video.muted = false;
        video.volume = 1;
        const audio = audioRef.current;
        if (audio) {
          audio.muted = false;
          audio.volume = 1;
        }
        void video.play().catch(() => {
          // Autoplay policy: start muted, prompt user to unmute.
          video.muted = true;
          if (audio) audio.muted = true;
          setNeedsUnmute(true);
          void video.play().catch(() => {});
          if (audio) void audio.play().catch(() => {});
        });
        if (audio && audioSrc) {
          void audio.play().catch(() => {});
        }
      },
      [audioSrc],
    );

    const loadHls = useCallback(
      async (video: HTMLVideoElement, url: string) => {
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
            hls.on(Hls.Events.MANIFEST_PARSED, (_event, data) => {
              // Explicitly select an audio track when the master lists separate ones.
              if (data.audioTracks && data.audioTracks.length > 0) {
                const preferred = data.audioTracks.find((t) => t.default) ?? data.audioTracks[0];
                hls.audioTrack = preferred.id;
              }
              if (autoPlay) {
                tryPlayWithAudio(video);
              }
            });
          } else if (video.canPlayType("application/vnd.apple.mpegurl")) {
            // Safari native HLS fallback
            video.src = url;
            if (autoPlay) tryPlayWithAudio(video);
          } else {
            video.src = url;
            if (autoPlay) tryPlayWithAudio(video);
          }
        } catch {
          video.src = url;
          if (autoPlay) tryPlayWithAudio(video);
        }
      },
      [autoPlay, tryPlayWithAudio],
    );

    useEffect(() => {
      const video = videoRef.current;
      if (!video || !src) return;
      setNeedsUnmute(false);

      if (isHls) {
        void loadHls(video, src);
      } else {
        video.src = src;
        if (autoPlay) tryPlayWithAudio(video);
      }

      return () => {
        if (hlsRef.current) {
          hlsRef.current.destroy();
          hlsRef.current = null;
        }
      };
    }, [src, isHls, loadHls, autoPlay, tryPlayWithAudio]);

    // Separate audio track (split A/V live cams): hidden <audio> element with
    // its own hls.js instance, driven in lock-step with the video element.
    useEffect(() => {
      const audio = audioRef.current;
      if (!audio) return;
      if (audioHlsRef.current) {
        audioHlsRef.current.destroy();
        audioHlsRef.current = null;
      }
      if (!audioSrc) {
        audio.removeAttribute("src");
        audio.load();
        return;
      }
      const lower = audioSrc.toLowerCase();
      const audioIsHls = lower.includes(".m3u8");
      // Mirror the video element's current mute/volume at attach time.
      const video = videoRef.current;
      if (video) {
        audio.muted = video.muted;
        audio.volume = video.volume;
      }
      let cancelled = false;
      if (audioIsHls) {
        void import("hls.js")
          .then(({ default: Hls }) => {
            if (cancelled) return;
            if (Hls.isSupported()) {
              const hls = new Hls({ enableWorker: true, lowLatencyMode: true });
              audioHlsRef.current = hls;
              hls.loadSource(audioSrc);
              hls.attachMedia(audio);
            } else {
              audio.src = audioSrc;
            }
          })
          .catch(() => {
            if (!cancelled) audio.src = audioSrc;
          });
      } else {
        audio.src = audioSrc;
      }
      return () => {
        cancelled = true;
        if (audioHlsRef.current) {
          audioHlsRef.current.destroy();
          audioHlsRef.current = null;
        }
      };
    }, [audioSrc]);

    const syncAudio = useCallback((action: "play" | "pause" | "volume") => {
      const audio = audioRef.current;
      const video = videoRef.current;
      if (!audio || !video) return;
      if (action === "play") {
        audio.muted = video.muted;
        audio.volume = video.volume;
        void audio.play().catch(() => {});
      } else if (action === "pause") {
        audio.pause();
      } else {
        audio.muted = video.muted;
        audio.volume = video.volume;
      }
    }, []);

    // Live A/V drift correction (throttled): re-seek the audio element when it
    // drifts more than ~1.5s from the video element.
    const handleAudioSync = useCallback(() => {
      const audio = audioRef.current;
      const video = videoRef.current;
      if (!audio || !video || audio.paused || !audioSrc) return;
      const now = Date.now();
      if (now - lastSyncRef.current < 4000) return;
      lastSyncRef.current = now;
      const vt = video.currentTime;
      const at = audio.currentTime;
      if (Number.isFinite(vt) && Number.isFinite(at) && Math.abs(vt - at) > 1.5) {
        try {
          audio.currentTime = vt;
        } catch {
          /* live edge not seekable yet — ignore */
        }
      }
    }, [audioSrc]);

    const handleVideoError = (e: React.SyntheticEvent<HTMLVideoElement>) => {
      const mediaError = e.currentTarget.error;
      if (mediaError?.code === 4) {
        console.warn("[HlsVideoPlayer] stream URL expired", { src });
      }
      onError?.(mediaError);
    };

    const handleUnmute = () => {
      const video = videoRef.current;
      if (!video) return;
      video.muted = false;
      video.volume = 1;
      const audio = audioRef.current;
      if (audio) {
        audio.muted = false;
        audio.volume = 1;
        void audio.play().catch(() => {});
      }
      setNeedsUnmute(false);
      void video.play().catch(() => {});
    };

    const handlePip = async () => {
      const video = videoRef.current;
      if (!video) return;
      try {
        if (document.pictureInPictureElement === video) {
          await document.exitPictureInPicture();
        } else {
          await video.requestPictureInPicture();
        }
      } catch (e) {
        console.warn("[HlsVideoPlayer] PiP failed", e);
      }
    };

    return (
      <div className="relative size-full min-h-0">
        <video
          ref={setVideoRef}
          key={src}
          // Progressive sources: set src on the element so playback works even if
          // the effect races. HLS must stay unset so hls.js owns attachment.
          src={isHls ? undefined : src}
          controls={controls}
          autoPlay={autoPlay}
          playsInline={playsInline}
          preload={preload}
          poster={poster}
          crossOrigin={crossOrigin}
          className={className}
          disablePictureInPicture={false}
          // Keep native controls tappable on Android (avoid overlay stealing hits).
          style={{ touchAction: "manipulation", ...style }}
          onError={handleVideoError}
          onTimeUpdate={onTimeUpdate}
          onPause={() => {
            syncAudio("pause");
            onPause?.();
          }}
          onPlay={() => {
            syncAudio("play");
            onPlay?.();
          }}
          onEnded={onEnded}
          onVolumeChange={() => {
            const video = videoRef.current;
            if (video && !video.muted && video.volume > 0) {
              setNeedsUnmute(false);
            }
            syncAudio("volume");
          }}
        >
          <track kind="captions" />
        </video>
        {/* Hidden companion audio for split A/V live streams. */}
        {audioSrc ? (
          <audio
            ref={(node) => {
              audioRef.current = node;
            }}
            key={audioSrc}
            preload="auto"
            playsInline
            className="hidden"
            aria-hidden="true"
            onTimeUpdate={handleAudioSync}
          />
        ) : null}

        {needsUnmute && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              handleUnmute();
            }}
            className="pointer-events-auto absolute bottom-14 left-1/2 z-10 flex -translate-x-1/2 items-center gap-2 rounded-full bg-black/80 px-4 py-2 text-sm text-white"
          >
            <VolumeX className="h-4 w-4" />
            Tap to unmute
          </button>
        )}

        {showPip && pipAvailable && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              void handlePip();
            }}
            title="Picture in picture"
            aria-label="Picture in picture"
            className="pointer-events-auto absolute right-2 top-2 z-10 rounded-md bg-black/70 p-2 text-white hover:bg-black/90"
          >
            <PictureInPicture2 className="h-4 w-4" />
          </button>
        )}
      </div>
    );
  },
);

HlsVideoPlayer.displayName = "HlsVideoPlayer";
