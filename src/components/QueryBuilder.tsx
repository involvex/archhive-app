import { useState } from "react";
import { Plus, X } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";

/** Query builder state: every group needs one of its terms; `exclude`
 * terms drop a video; `boost` terms only rank it higher. */
export interface BuilderQuery {
  groups: string[][];
  exclude: string[];
  boost: string[];
}

export const EMPTY_BUILDER: BuilderQuery = { groups: [], exclude: [], boost: [] };

/** The query as text: `(a OR b) AND (c) NOT (d OR e)`, plus boosts. */
export function builderExpression(q: BuilderQuery): string {
  const quote = (t: string) => (t.includes(" ") ? `"${t}"` : t);
  const parts = q.groups.filter((g) => g.length > 0).map((g) => `(${g.map(quote).join(" OR ")})`);
  let out = parts.join(" AND ");
  if (q.exclude.length) out += `${out ? " " : ""}NOT (${q.exclude.map(quote).join(" OR ")})`;
  if (q.boost.length) out += `${out ? " · " : ""}rank higher: ${q.boost.map(quote).join(", ")}`;
  return out;
}

export function builderTermCount(q: BuilderQuery): number {
  return q.groups.reduce((n, g) => n + g.length, 0) + q.exclude.length + q.boost.length;
}

/** Terms typed into a box: comma-separated, so a phrase keeps its spaces. */
function splitTerms(raw: string): string[] {
  return raw
    .split(",")
    .map((t) => t.trim())
    .filter(Boolean);
}

function TermList({
  terms,
  onChange,
  placeholder,
  tone,
}: {
  terms: string[];
  onChange: (terms: string[]) => void;
  placeholder: string;
  tone: "group" | "exclude" | "boost";
}) {
  const [draft, setDraft] = useState("");
  const add = () => {
    const more = splitTerms(draft).filter((t) => !terms.includes(t));
    if (more.length) onChange([...terms, ...more]);
    setDraft("");
  };
  const chip =
    tone === "exclude"
      ? "bg-red-500/15 text-red-300"
      : tone === "boost"
        ? "bg-amber-500/15 text-amber-300"
        : "bg-[var(--color-secondary)]";
  return (
    <div className="flex flex-wrap items-center gap-1.5">
      {terms.map((t, i) => (
        <span key={t} className="inline-flex items-center gap-1.5">
          {tone !== "boost" && i > 0 && (
            <span className="text-[10px] font-semibold text-[var(--color-muted-foreground)]">
              OR
            </span>
          )}
          <span
            className={`inline-flex items-center gap-1 rounded-full px-2.5 py-1 text-xs ${chip}`}
          >
            {tone === "exclude" ? "−" : tone === "boost" ? "+" : ""}
            {t}
            <button
              type="button"
              aria-label={`Remove ${t}`}
              onClick={() => onChange(terms.filter((x) => x !== t))}
              className="opacity-70 hover:opacity-100"
            >
              <X className="h-3 w-3" />
            </button>
          </span>
        </span>
      ))}
      <Input
        className="h-8 min-w-[9rem] flex-1 text-xs"
        placeholder={placeholder}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            add();
          }
        }}
        onBlur={() => draft.trim() && add()}
      />
    </div>
  );
}

/**
 * Groups of alternatives (OR inside a group, AND between groups), an
 * exclusion list and optional "rank higher" terms. Terms are whole words or
 * phrases in titles, tags and performers; `term*` matches as a prefix.
 */
export function QueryBuilder({
  value,
  onChange,
}: {
  value: BuilderQuery;
  onChange: (q: BuilderQuery) => void;
}) {
  const setGroup = (i: number, terms: string[]) => {
    const groups = value.groups.map((g, j) => (j === i ? terms : g));
    onChange({ ...value, groups });
  };
  return (
    <div className="space-y-3 rounded-md border border-[var(--color-border)] p-2">
      {builderTermCount(value) > 0 && (
        <p className="break-words rounded bg-[var(--color-muted)] px-2 py-1 font-mono text-[11px]">
          {builderExpression(value)}
        </p>
      )}
      <p className="text-[11px] text-[var(--color-muted-foreground)]">
        Each group needs at least one of its terms (synonyms, similar tags); all groups must match.
        Terms are words or phrases in titles, tags and performers; end with * for a prefix (tit* =
        tits, titty). Separate several with commas.
      </p>
      {value.groups.map((g, i) => (
        <div key={i} className="space-y-1">
          <div className="flex items-center justify-between text-[11px] font-medium">
            <span>{i === 0 ? "Any of" : "AND any of"}</span>
            <button
              type="button"
              className="text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)]"
              onClick={() => onChange({ ...value, groups: value.groups.filter((_, j) => j !== i) })}
            >
              Remove group
            </button>
          </div>
          <TermList
            terms={g}
            onChange={(t) => setGroup(i, t)}
            placeholder="e.g. blowjob, bj, oral"
            tone="group"
          />
        </div>
      ))}
      <Button
        size="sm"
        variant="outline"
        onClick={() => onChange({ ...value, groups: [...value.groups, []] })}
      >
        <Plus className="h-3.5 w-3.5" />
        {value.groups.length ? "AND group" : "Add group"}
      </Button>
      <div className="space-y-1">
        <p className="text-[11px] font-medium">Exclude (none of)</p>
        <TermList
          terms={value.exclude}
          onChange={(exclude) => onChange({ ...value, exclude })}
          placeholder="e.g. solo, compilation"
          tone="exclude"
        />
      </div>
      <div className="space-y-1">
        <p className="text-[11px] font-medium">Rank higher (optional)</p>
        <TermList
          terms={value.boost}
          onChange={(boost) => onChange({ ...value, boost })}
          placeholder="e.g. pov, 4k"
          tone="boost"
        />
      </div>
      {builderTermCount(value) > 0 && (
        <Button size="sm" variant="ghost" onClick={() => onChange(EMPTY_BUILDER)}>
          Clear query
        </Button>
      )}
    </div>
  );
}
