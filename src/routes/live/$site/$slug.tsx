import { createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import { Button } from "@/components/ui/button";
import { HlsVideoPlayer } from "@/components/HlsVideoPlayer";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Radio, ArrowLeft } from "lucide-react";
import { Link } from "@tanstack/react-router";

export const Route = createFileRoute("/live/$site/$slug")({
  component: LivePlayerPage,
});

const SITE_LABELS: Record<string, string> = {
  chaturbate: "Chaturbate",
  stripchat: "Stripchat",
};

function LivePlayerPage() {
  const { site, slug } = Route.useParams();

  const [streamUrl, setStreamUrl] = useState("");
  const [embedUrl, setEmbedUrl] = useState("");
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
      setEmbedUrl(result.embed_url);
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
                  autoPlay
                  playsInline
                  className="h-full w-full object-contain"
                  onError={(mediaError: MediaError | null) => {
                    if (mediaError?.code === 4) {
                      setError(
                        "Playback failed — the stream URL may have expired. Tap Retry for a fresh one.",
                      );
                    } else {
                      setError("Playback failed.");
                    }
                  }}
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
                <CardTitle className="text-sm">Live Chat</CardTitle>
                <Button variant="ghost" size="sm" onClick={() => setShowChat(!showChat)}>
                  {showChat ? "Hide" : "Show"}
                </Button>
              </div>
            </CardHeader>
            <CardContent className="p-0">
              {showChat && embedUrl ? (
                <iframe
                  src={embedUrl}
                  className="w-full h-[500px] border-0"
                  title={`${slug} chat`}
                  allow="autoplay; encrypted-media"
                />
              ) : (
                <div className="p-4 text-sm text-[var(--color-muted-foreground)]">
                  Click "Show" to open the embedded chat.
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
