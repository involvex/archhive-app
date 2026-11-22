/** A `[mm:ss]` / `[h:mm:ss]` timestamp found inside scene notes. */
export interface NoteTimestamp {
  /** Character offset of the leading `[`. */
  index: number;
  /** Raw match including brackets, e.g. `[1:23]`. */
  raw: string;
  /** Seek target in whole seconds. */
  seconds: number;
  /** Display label without brackets, e.g. `1:23`. */
  label: string;
}

const TIMESTAMP_RE = /\[(\d{1,2}):([0-5]?\d)(?::([0-5]?\d))?\]/g;

/** Parse every `[mm:ss]` / `[h:mm:ss]` marker in notes text. */
export function parseNoteTimestamps(notes: string | undefined | null): NoteTimestamp[] {
  if (!notes) return [];
  const out: NoteTimestamp[] = [];
  TIMESTAMP_RE.lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = TIMESTAMP_RE.exec(notes)) !== null) {
    const a = Number(match[1]);
    const b = Number(match[2]);
    const c = match[3] != null ? Number(match[3]) : null;
    let seconds: number;
    let label: string;
    if (c != null) {
      seconds = a * 3600 + b * 60 + c;
      label = `${a}:${String(b).padStart(2, "0")}:${String(c).padStart(2, "0")}`;
    } else {
      seconds = a * 60 + b;
      label = `${a}:${String(b).padStart(2, "0")}`;
    }
    out.push({
      index: match.index,
      raw: match[0],
      seconds,
      label,
    });
  }
  return out;
}

export type NoteSegment =
  | { type: "text"; text: string }
  | { type: "timestamp"; text: string; seconds: number; label: string };

/** Split notes into plain text and clickable timestamp segments (in order). */
export function splitNotesWithTimestamps(notes: string | undefined | null): NoteSegment[] {
  if (!notes) return [];
  const stamps = parseNoteTimestamps(notes);
  if (stamps.length === 0) return [{ type: "text", text: notes }];

  const segments: NoteSegment[] = [];
  let cursor = 0;
  for (const stamp of stamps) {
    if (stamp.index > cursor) {
      segments.push({ type: "text", text: notes.slice(cursor, stamp.index) });
    }
    segments.push({
      type: "timestamp",
      text: stamp.raw,
      seconds: stamp.seconds,
      label: stamp.label,
    });
    cursor = stamp.index + stamp.raw.length;
  }
  if (cursor < notes.length) {
    segments.push({ type: "text", text: notes.slice(cursor) });
  }
  return segments;
}
