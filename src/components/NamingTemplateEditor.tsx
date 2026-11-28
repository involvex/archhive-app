import { useCallback, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  BUILTIN_NAMING_PRESETS,
  deleteNamingPreset,
  findPresetById,
  loadNamingPresets,
  renameNamingPreset,
  saveNamingPreset,
  type NamingPreset,
} from "@/lib/namingPresets";
import {
  analyseNamingTemplate,
  insertNamingToken,
  NAMING_TOKENS,
  type NamingToken,
} from "@/lib/namingTemplate";

export interface NamingTemplateEditorProps {
  value: string;
  onChange: (template: string) => void;
  disabled?: boolean;
}

export function NamingTemplateEditor({
  value,
  onChange,
  disabled = false,
}: NamingTemplateEditorProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [presets, setPresets] = useState<NamingPreset[]>(() => loadNamingPresets());
  const [selectedId, setSelectedId] = useState("");
  const [presetName, setPresetName] = useState("");
  const [status, setStatus] = useState<string | null>(null);

  const naming = analyseNamingTemplate(value);
  const selectedIsUser = presets.some((p) => p.id === selectedId);

  const applyToken = useCallback(
    (token: NamingToken) => {
      if (disabled) return;
      const el = inputRef.current;
      const start = el?.selectionStart ?? value.length;
      const end = el?.selectionEnd ?? start;
      const { value: next, caret } = insertNamingToken(value, token, start, end);
      onChange(next);
      requestAnimationFrame(() => {
        const input = inputRef.current;
        if (!input) return;
        input.focus();
        input.setSelectionRange(caret, caret);
      });
    },
    [disabled, onChange, value],
  );

  function handleSelectPreset(id: string) {
    setSelectedId(id);
    setStatus(null);
    if (!id) {
      setPresetName("");
      return;
    }
    const preset = findPresetById(id);
    if (!preset) return;
    onChange(preset.template);
    setPresetName("builtin" in preset && preset.builtin ? "" : preset.name);
  }

  function handleSave() {
    const next = saveNamingPreset(presetName, value);
    if (!next) {
      setStatus("Enter a preset name to save.");
      return;
    }
    setPresets(next);
    const match = next.find((p) => p.name.toLowerCase() === presetName.trim().toLowerCase());
    if (match) setSelectedId(match.id);
    setStatus(`Saved “${presetName.trim()}”.`);
  }

  function handleRename() {
    if (!selectedIsUser) {
      setStatus("Pick a saved preset to rename.");
      return;
    }
    const next = renameNamingPreset(selectedId, presetName);
    if (!next) {
      setStatus("Enter a new name to rename.");
      return;
    }
    setPresets(next);
    setStatus(`Renamed to “${presetName.trim()}”.`);
  }

  function handleDelete() {
    if (!selectedIsUser) {
      setStatus("Pick a saved preset to delete.");
      return;
    }
    const next = deleteNamingPreset(selectedId);
    setPresets(next);
    setSelectedId("");
    setPresetName("");
    setStatus("Preset deleted.");
  }

  return (
    <div className="space-y-2">
      <label className="text-xs text-[var(--color-muted-foreground)]" htmlFor="naming-template">
        Naming template
      </label>
      <Input
        id="naming-template"
        ref={inputRef}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        className="font-mono text-sm"
        aria-label="Naming template"
      />

      <div className="flex flex-wrap gap-1.5" role="group" aria-label="Insert naming token">
        {NAMING_TOKENS.map((token) => (
          <button
            key={token}
            type="button"
            disabled={disabled}
            onClick={() => applyToken(token)}
            className="min-h-9 rounded-full bg-[var(--color-secondary)] px-3 py-1 text-xs font-medium text-[var(--color-secondary-foreground)] transition hover:bg-[var(--color-muted)] disabled:opacity-50"
          >
            {`{${token}}`}
          </button>
        ))}
      </div>

      <div className="space-y-2 rounded-md border border-[var(--color-border)] p-3">
        <p className="text-xs font-medium text-[var(--color-foreground)]">Presets</p>
        <select
          className="h-9 w-full rounded-md border border-[var(--color-input)] bg-[var(--color-background)] px-3 text-sm"
          value={selectedId}
          disabled={disabled}
          onChange={(e) => handleSelectPreset(e.target.value)}
          aria-label="Load naming preset"
        >
          <option value="">Load a preset…</option>
          <optgroup label="Built-in">
            {BUILTIN_NAMING_PRESETS.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </optgroup>
          {presets.length > 0 && (
            <optgroup label="Saved">
              {presets.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </optgroup>
          )}
        </select>
        <div className="flex flex-wrap items-center gap-2">
          <Input
            value={presetName}
            onChange={(e) => setPresetName(e.target.value)}
            disabled={disabled}
            placeholder="Preset name"
            className="min-h-9 flex-1"
            aria-label="Preset name"
          />
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="min-h-9"
            disabled={disabled}
            onClick={handleSave}
          >
            Save
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="min-h-9"
            disabled={disabled || !selectedIsUser}
            onClick={handleRename}
          >
            Rename
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="min-h-9"
            disabled={disabled || !selectedIsUser}
            onClick={handleDelete}
          >
            Delete
          </Button>
        </div>
        {status && <p className="text-[10px] text-[var(--color-muted-foreground)]">{status}</p>}
      </div>

      <div className="space-y-1 text-[10px] text-[var(--color-muted-foreground)]">
        <p>
          Placeholders: {"{performer}"}, {"{title}"}, {"{ext}"}, {"{id}"}, {"{site}"}, {"{date}"},{" "}
          {"{channel}"}. Example: {"{performer}/{title}.{ext}"}
        </p>
        <p className="font-mono text-[var(--color-foreground)]">Preview: {naming.preview}</p>
        {naming.unknownTokens.length > 0 && (
          <p className="text-amber-400">
            Unknown placeholder
            {naming.unknownTokens.length === 1 ? "" : "s"}:{" "}
            {naming.unknownTokens.map((t) => `{${t}}`).join(", ")}
            {naming.fallsBackToDefault ? " — falls back to {performer}/{title}.{ext}" : ""}
          </p>
        )}
        {naming.hasIllegalChars && (
          <p className="text-red-400">
            Contains characters illegal in Windows paths (&lt; &gt; : &quot; | ? *).
          </p>
        )}
      </div>
    </div>
  );
}
