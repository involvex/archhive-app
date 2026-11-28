/** Tokens ArcHive naming templates accept (mirrors downloads/naming.rs). */
export const NAMING_TOKENS = [
  "performer",
  "channel",
  "uploader",
  "title",
  "ext",
  "id",
  "site",
  "extractor",
  "date",
] as const;

export type NamingToken = (typeof NAMING_TOKENS)[number];

const TOKEN_TO_SAMPLE: Record<NamingToken, string> = {
  performer: "JaneDoe",
  channel: "JaneDoe",
  uploader: "JaneDoe",
  title: "Sample Title",
  ext: "mp4",
  id: "abc123",
  site: "pornhub",
  extractor: "pornhub",
  date: "20261122",
};

const TOKEN_TO_YTDLP: Record<NamingToken, string> = {
  performer: "%(uploader)s",
  channel: "%(uploader)s",
  uploader: "%(uploader)s",
  title: "%(title)s",
  ext: "%(ext)s",
  id: "%(id)s",
  site: "%(extractor)s",
  extractor: "%(extractor)s",
  date: "%(upload_date)s",
};

const BRACE_TOKEN_RE = /\{([A-Za-z_]+)\}/g;

/** Characters illegal in Windows file paths (and commonly blocked elsewhere). */
const ILLEGAL_PATH_CHARS = /[<>:"|?*]/;

function hasControlChars(value: string): boolean {
  for (let i = 0; i < value.length; i++) {
    const code = value.charCodeAt(i);
    if (code <= 0x1f) return true;
  }
  return false;
}

export interface NamingTemplateAnalysis {
  /** Sample rendered path using placeholder values. */
  preview: string;
  /** Brace tokens that are not in the known map. */
  unknownTokens: string[];
  /** True when the template contains characters illegal on Windows paths. */
  hasIllegalChars: boolean;
  /** True when the template falls back to the safe yt-dlp default. */
  fallsBackToDefault: boolean;
  /** Converted yt-dlp `-o` template (or the safe default). */
  ytdlpTemplate: string;
}

function knownToken(name: string): NamingToken | null {
  const lower = name.toLowerCase() as NamingToken;
  if ((NAMING_TOKENS as readonly string[]).includes(lower)) return lower;
  return null;
}

/** Analyse a user naming template for preview, unknown tokens, and illegal chars. */
export function analyseNamingTemplate(template: string | undefined | null): NamingTemplateAnalysis {
  const trimmed = (template ?? "").trim();
  const empty = trimmed.length === 0;
  const source = empty ? "{performer}/{title}.{ext}" : trimmed;

  const unknownTokens: string[] = [];
  BRACE_TOKEN_RE.lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = BRACE_TOKEN_RE.exec(source)) !== null) {
    if (!knownToken(match[1])) {
      unknownTokens.push(match[1]);
    }
  }

  // Already a yt-dlp template — show as-is for preview.
  if (source.includes("%(")) {
    return {
      preview: source,
      unknownTokens,
      hasIllegalChars: ILLEGAL_PATH_CHARS.test(source) || hasControlChars(source),
      fallsBackToDefault: false,
      ytdlpTemplate: source,
    };
  }

  let preview = source;
  let ytdlp = source;
  for (const token of NAMING_TOKENS) {
    const brace = `{${token}}`;
    const braceTitle = `{${token[0].toUpperCase()}${token.slice(1)}}`;
    preview = preview.split(brace).join(TOKEN_TO_SAMPLE[token]);
    preview = preview.split(braceTitle).join(TOKEN_TO_SAMPLE[token]);
    ytdlp = ytdlp.split(brace).join(TOKEN_TO_YTDLP[token]);
    ytdlp = ytdlp.split(braceTitle).join(TOKEN_TO_YTDLP[token]);
  }

  // Strip remaining unknown braces from the sample so the preview stays readable.
  preview = preview.replace(BRACE_TOKEN_RE, "_");

  const fallsBackToDefault = !ytdlp.includes("%(");
  if (fallsBackToDefault) {
    ytdlp = "%(uploader)s/%(title)s.%(ext)s";
    preview = "JaneDoe/Sample Title.mp4";
  }

  return {
    preview,
    unknownTokens: [...new Set(unknownTokens)],
    hasIllegalChars: ILLEGAL_PATH_CHARS.test(source) || hasControlChars(source),
    fallsBackToDefault,
    ytdlpTemplate: ytdlp,
  };
}

export interface InsertNamingTokenResult {
  /** Template after splicing `{token}` at the selection. */
  value: string;
  /** Caret position after the inserted token (for re-focus). */
  caret: number;
}

/**
 * Insert `{token}` at `[start, end)` in the template (replaces a selection).
 * Out-of-range indices clamp to the string bounds.
 */
export function insertNamingToken(
  template: string,
  token: NamingToken,
  start: number,
  end: number = start,
): InsertNamingTokenResult {
  const value = template ?? "";
  const lo = Math.max(0, Math.min(start, end, value.length));
  const hi = Math.max(0, Math.min(Math.max(start, end), value.length));
  const insertion = `{${token}}`;
  return {
    value: value.slice(0, lo) + insertion + value.slice(hi),
    caret: lo + insertion.length,
  };
}
