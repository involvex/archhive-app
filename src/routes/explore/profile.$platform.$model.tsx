import { createFileRoute, Link } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { CamgirlfinderProfile, MediaItem } from "@/lib/types";
import { SceneCard } from "@/components/SceneCard";
import { ErrorState } from "@/components/ErrorState";
import { SkeletonGrid } from "@/components/SkeletonGrid";
import { Button } from "@/components/ui/button";
import { ArrowLeft, ExternalLink, Radio } from "lucide-react";
import { openExternal } from "@/lib/external";

export const Route = createFileRoute("/explore/profile/$platform/$model")({
  component: ExploreModelPage,
});

function liveSiteForPlatform(platform: string): "chaturbate" | "stripchat" | null {
  if (platform === "cb") return "chaturbate";
  if (platform === "sc") return "stripchat";
  return null;
}

function ExploreModelPage() {
  const { platform, model } = Route.useParams();
  const [profile, setProfile] = useState<CamgirlfinderProfile | null>(null);
  const [similar, setSimilar] = useState<MediaItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const [p, sim] = await Promise.all([
        api.camgirlfinderGetModel(platform, model),
        api.camgirlfinderSimilar(platform, model),
      ]);
      setProfile(p);
      setSimilar(sim);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load profile");
    } finally {
      setLoading(false);
    }
  }, [platform, model]);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect -- fetch profile on route params
    void load();
  }, [load]);

  const liveSite = liveSiteForPlatform(platform);
  const playable = profile?.playable_url;
  const hero = profile?.persons[0]?.full_image ?? profile?.persons[0]?.face_image ?? null;

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <Button asChild variant="ghost" size="sm">
          <Link to="/explore">
            <ArrowLeft className="mr-1 h-4 w-4" />
            Explore
          </Link>
        </Button>
      </div>

      {error && <ErrorState message={error} onRetry={() => void load()} />}
      {loading && <SkeletonGrid count={4} cols={2} />}

      {profile && !loading && (
        <>
          <div className="flex flex-col gap-4 sm:flex-row">
            {hero && (
              <img
                src={hero}
                alt={profile.name}
                className="h-48 w-48 rounded-xl object-cover sm:h-56 sm:w-56"
              />
            )}
            <div className="space-y-2">
              <h2 className="text-2xl font-bold">{profile.name}</h2>
              <p className="text-sm text-[var(--color-muted-foreground)]">
                {profile.platform.toUpperCase()} · {profile.gender}
                {profile.last_seen ? ` · last seen ${profile.last_seen}` : ""}
              </p>
              <div className="flex flex-wrap gap-2 pt-2">
                {liveSite && playable && (
                  <Button asChild>
                    <Link
                      to="/live/$site/$slug"
                      params={{ site: liveSite, slug: profile.name }}
                      search={{
                        fromSearch: "0",
                        searchQuery: "",
                        roomUrl: playable,
                      }}
                    >
                      <Radio className="mr-1.5 h-4 w-4" />
                      Open room
                    </Link>
                  </Button>
                )}
                <Button
                  type="button"
                  variant="secondary"
                  onClick={() => void openExternal(profile.external_profile).catch(console.error)}
                >
                  <ExternalLink className="mr-1.5 h-4 w-4" />
                  Open affiliate link
                </Button>
              </div>
            </div>
          </div>

          {profile.persons.length > 1 && (
            <div>
              <h3 className="mb-2 text-lg font-semibold">Faces</h3>
              <div className="flex flex-wrap gap-2">
                {profile.persons.map((p) => (
                  <img
                    key={p.person}
                    src={p.face_image ?? p.full_image ?? ""}
                    alt={`Person ${p.person}`}
                    className="h-20 w-20 rounded-lg object-cover"
                  />
                ))}
              </div>
            </div>
          )}

          <div>
            <h3 className="mb-2 text-lg font-semibold">Similar</h3>
            {similar.length === 0 ? (
              <p className="text-sm text-[var(--color-muted-foreground)]">
                No similar accounts found.
              </p>
            ) : (
              <div className="grid grid-cols-2 gap-2.5 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5">
                {similar.map((item) => {
                  const plat =
                    item.tags.find((t) => t.startsWith("platform:"))?.slice("platform:".length) ??
                    "cb";
                  const site = liveSiteForPlatform(plat);
                  const slug = item.channel ?? item.performers[0] ?? item.title;
                  if (site) {
                    return (
                      <Link
                        key={item.id}
                        to="/live/$site/$slug"
                        params={{ site, slug }}
                        search={{
                          fromSearch: "0",
                          searchQuery: "",
                          roomUrl: item.url,
                        }}
                      >
                        <SceneCard item={item} />
                      </Link>
                    );
                  }
                  return (
                    <Link
                      key={item.id}
                      to="/explore/profile/$platform/$model"
                      params={{ platform: plat, model: slug }}
                    >
                      <SceneCard item={item} />
                    </Link>
                  );
                })}
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}
