import { createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { api, isRoomOfflineError } from "@/lib/api/client";
import { Button } from "@/components/ui/button";
import { HlsVideoPlayer, STREAM_URL_EXPIRED_ERROR } from "@/components/HlsVideoPlayer";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Radio, ArrowLeft, ExternalLink } from "lucide-react";
import { Link } from "@tanstack/react-router";

export const Route = createFileRoute("/live/$site/$slug")({
  component: LivePlayerPage,
});

const SITE_LABELS: Record<string, string> = {
  chaturbate: "Chaturbate",
  stripchat: "Stripchat",
};

/**
 * Get the chat URL for a live stream.
 * NOTE: Chaturbate/Stripchat room pages send `X-Frame-Options` / CSP
 * `frame-ancestors` headers, so they render blank inside an <iframe>.
 * Use this URL for an "open in browser" link, not an embedded frame.
 */
function getChatUrl(site: string, slug: string): string {
  // Chaturbate chat is at the regular room URL, not the embed URL.
  // The embed URL (with embed_video_only=1) is for video-only playback.
  switch (site) {
    case "chaturbate":
      // Use the regular room URL for chat - this includes the chat interface
      return `https://chaturbate.com/${slug}/`;
    case "stripchat":
      return `https://stripchat.com/${slug}/`;
    default:
      return "";
  }
}

function LivePlayerPage() {
  const { site, slug } = Route.useParams();

  const [streamUrl, setStreamUrl] = useState("");
  const [audioUrl, setAudioUrl] = useState("");
  const [embedUrl, setEmbedUrl] = useState("");
  const [streamError, setStreamError] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [showChat, setShowChat] = useState(false);

  const loadStream = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      // Build room URL generically for supported live sites
      const roomUrl = `https://${site}.com/${slug}/`;
      const result = await api.resolveLivestream(roomUrl);
      setStreamUrl(result.stream_url);
      setAudioUrl(result.audio_url);
      setEmbedUrl(result.embed_url);
      setStreamError(result.stream_error);
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Failed to resolve stream";
      setError(msg.replace(/^site error:\s*/i, ""));
    } finally {
      setLoading(false);
    }
  }, [site, slug]);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void loadStream();
  }, [loadStream]);

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-3">
        <Button asChild variant="ghost" size="sm">
          <Link to="/live">
            <ArrowLeft className="h-4 w-4 mr-1" />
            Back
          </Link>
        </Button>
        <Radio className="h-5 w-5 text-red-500" />
        <h2 className="text-2xl font-bold">
          {slug}
          <span className="ml-2 text-sm font-normal text-[var(--color-muted-foreground)]">
            {SITE_LABELS[site] ?? site}
          </span>
        </h2>
      </div>

      {!streamUrl && embedUrl && !isRoomOfflineError(streamError) && (
        <div className="space-y-1 text-xs text-[var(--color-muted-foreground)]">
          <p>
            Playing the site embed player (this cam serves separate audio/video streams that
            browsers can&apos;t mux directly). Retry for direct HLS if cookies are configured.
          </p>
          {streamError && <p className="font-mono break-all">Direct HLS failed: {streamError}</p>}
          <a
            href={getChatUrl(site, slug)}
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center gap-1 text-xs text-[var(--color-primary)] underline underline-offset-2"
          >
            <ExternalLink className="h-3 w-3" />
            Open room in browser (full controls + audio)
          </a>
        </div>
      )}

      {!streamUrl && isRoomOfflineError(streamError) && (
        <div className="rounded-md border border-amber-400/30 bg-amber-400/10 px-3 py-2 text-sm text-amber-300">
          This room looks offline or is in a private show right now — nothing to play.
        </div>
      )}

      {error && (
        <div className="flex items-center justify-between rounded-md border border-red-400/30 bg-red-400/10 px-3 py-2 text-sm text-red-400">
          <p>{error}</p>
          <Button variant="ghost" size="sm" onClick={() => void loadStream()}>
            Retry
          </Button>
        </div>
      )}

      <div className="flex gap-4 flex-col lg:flex-row">
        {/* Video player */}
        <div className="flex-1">
          <Card className="overflow-hidden">
            <div className="aspect-video bg-black relative">
              {loading ? (
                <div className="flex h-full items-center justify-center text-white/60">
                  Resolving stream...
                </div>
              ) : streamUrl ? (
                <HlsVideoPlayer
                  src={streamUrl}
                  audioSrc={audioUrl || undefined}
                  autoPlay
                  playsInline
                  className="h-full w-full object-contain"
                  onError={(mediaError: MediaError | null) => {
                    if (mediaError?.code === 4) {
                      setError(STREAM_URL_EXPIRED_ERROR);
                    } else {
                      setError("Playback failed.");
                    }
                  }}
                />
              ) : embedUrl && !isRoomOfflineError(streamError) ? (
                <iframe
                  src={embedUrl}
                  className="h-full w-full border-0"
                  style={{ pointerEvents: "auto", touchAction: "manipulation" }}
                  title={`${slug} live stream`}
                  allow="autoplay; encrypted-media; fullscreen; picture-in-picture"
                  allowFullScreen
                />
              ) : (
                <div className="flex h-full items-center justify-center text-white/60">
                  No stream available
                </div>
              )}
            </div>
          </Card>
        </div>

        {/* Chat / Embed panel */}
        <div className="w-full lg:w-96">
          <Card className="h-full">
            <CardHeader className="pb-2">
              <div className="flex items-center justify-between">
                <CardTitle className="text-sm">
                  {site === "chaturbate" ? "Live Chat" : "Chat / Stream"}
                </CardTitle>
                <Button variant="ghost" size="sm" onClick={() => setShowChat(!showChat)}>
                  {showChat ? "Hide" : "Show"}
                </Button>
              </div>
            </CardHeader>
            <CardContent className="p-0">
              {showChat ? (
                // Room pages block framing (X-Frame-Options), so link out
                // instead of embedding a blank frame.
                <div className="space-y-2 p-4">
                  <p className="text-sm text-[var(--color-muted-foreground)]">
                    Chat can&apos;t be embedded here — Chaturbate blocks framing of room pages. Open
                    it in the browser:
                  </p>
                  <Button asChild variant="outline" size="sm">
                    <a href={getChatUrl(site, slug)} target="_blank" rel="noreferrer">
                      <ExternalLink className="mr-1 h-4 w-4" />
                      Open live chat
                    </a>
                  </Button>
                  <p className="break-all font-mono text-xs text-[var(--color-muted-foreground)]">
                    {getChatUrl(site, slug)}
                  </p>
                </div>
              ) : (
                <div className="p-4 text-sm text-[var(--color-muted-foreground)]">
                  Click "Show" to open the chat for{" "}
                  {site === "chaturbate" ? "Chaturbate" : "Stripchat"}.
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
