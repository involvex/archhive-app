import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "@/lib/api/client";
import type { Performer } from "@/lib/types";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Filter, Users, Download, Camera } from "lucide-react";

export const Route = createFileRoute("/library/performers/")({
  component: PerformersPage,
});

type PerformerSort = "name" | "scenes";

function PerformersPage() {
  const navigate = useNavigate();
  const [performers, setPerformers] = useState<Performer[]>([]);
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<PerformerSort>("name");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [uploadTargetId, setUploadTargetId] = useState<string | null>(null);

  useEffect(() => {
    void api
      .listPerformers(query || undefined)
      .then(setPerformers)
      .catch(console.error);
  }, [query]);

  function toggle(name: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(name)) {
        next.delete(name);
      } else {
        next.add(name);
      }
      return next;
    });
  }

  function applyFilter() {
    if (selected.size === 0) return;
    navigate({
      to: "/library/scenes",
      search: { performers: [...selected] },
    });
  }

  async function exportPerformers() {
    try {
      const data = await api.exportPerformers();
      const json = JSON.stringify(data, null, 2);
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "performers.json";
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      console.error("Export failed", e);
    }
  }

  const handleImageUpload = useCallback(
    async (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (!file || !uploadTargetId) return;
      try {
        const reader = new FileReader();
        reader.onload = async () => {
          const dataUrl = reader.result as string;
          await api.setPerformerImage(uploadTargetId, dataUrl);
          setPerformers((prev) =>
            prev.map((p) => (p.id === uploadTargetId ? { ...p, image: dataUrl } : p)),
          );
        };
        reader.readAsDataURL(file);
      } catch (err) {
        console.error("Image upload failed", err);
      }
      e.target.value = "";
    },
    [uploadTargetId],
  );

  return (
    <div className="space-y-4">
      <input
        ref={fileInputRef}
        type="file"
        accept="image/*"
        className="hidden"
        onChange={handleImageUpload}
      />
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold">Performers</h2>
        <div className="flex gap-2">
          <Button size="sm" variant="outline" onClick={exportPerformers}>
            <Download className="mr-1 h-3.5 w-3.5" />
            Export
          </Button>
          {selected.size > 0 && (
            <Button size="sm" onClick={applyFilter}>
              <Filter className="mr-1 h-3.5 w-3.5" />
              Filter scenes ({selected.size})
            </Button>
          )}
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <Input
          placeholder="Search performers..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          className="max-w-md flex-1"
        />
        <div className="flex gap-1" title="Sort performers">
          {(["name", "scenes"] as const).map((mode) => (
            <Button
              key={mode}
              size="sm"
              variant={sort === mode ? "default" : "outline"}
              onClick={() => setSort(mode)}
            >
              {mode === "name" ? "Name" : "Scenes"}
            </Button>
          ))}
        </div>
      </div>
      <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
        {[...performers]
          .sort((a, b) =>
            sort === "scenes"
              ? b.scene_count - a.scene_count || a.name.localeCompare(b.name)
              : a.name.localeCompare(b.name),
          )
          .map((p) => {
            const checked = selected.has(p.name);
            return (
              <Card
                key={p.id}
                className={`cursor-pointer transition-colors ${
                  checked
                    ? "border-[var(--color-primary)] bg-[var(--color-primary)]/5"
                    : "hover:border-[var(--color-primary)]"
                }`}
                onClick={() => toggle(p.name)}
              >
                <CardContent className="flex items-center gap-3 p-3">
                  <button
                    type="button"
                    className="group relative flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-[var(--color-muted)] overflow-hidden cursor-pointer"
                    onClick={(e) => {
                      e.stopPropagation();
                      setUploadTargetId(p.id);
                      fileInputRef.current?.click();
                    }}
                    title="Set profile image"
                  >
                    {p.image ? (
                      <img src={p.image} alt={p.name} className="h-full w-full object-cover" />
                    ) : (
                      <Users className="h-5 w-5" />
                    )}
                    <div className="absolute inset-0 flex items-center justify-center bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity">
                      <Camera className="h-4 w-4 text-white" />
                    </div>
                  </button>
                  <div>
                    <p className="font-medium">{p.name}</p>
                    <p className="text-xs text-[var(--color-muted-foreground)]">
                      {p.scene_count} scenes
                    </p>
                  </div>
                </CardContent>
              </Card>
            );
          })}
      </div>
      {performers.length === 0 && (
        <p className="text-sm text-[var(--color-muted-foreground)]">No performers yet.</p>
      )}
    </div>
  );
}
