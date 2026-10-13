import type { StashSceneMatch } from "@/lib/types";
import { cn } from "@/lib/utils";

/** "92% · Riley Reid, same length": how well a StashDB candidate fits. */
export function StashMatchScore({
  match,
  className,
}: {
  match: StashSceneMatch;
  className?: string;
}) {
  if (match.score == null) return null;
  const pct = Math.round(match.score * 100);
  const tone =
    match.score >= 0.85 ? "text-green-400" : match.score >= 0.6 ? "text-amber-400" : "opacity-70";
  return (
    <span className={cn("block truncate text-xs", tone, className)}>
      {pct}% match{match.match_reasons?.length ? ` · ${match.match_reasons.join(", ")}` : ""}
    </span>
  );
}
