import type { DuplicateGroup, Scene } from "@/lib/types";
import { Button } from "@/components/ui/button";

interface DuplicateGroupCardProps {
  group: DuplicateGroup;
  groupKey: string;
  selectedId: string | undefined;
  deleteFiles: boolean;
  onSelect: (sceneId: string) => void;
  onMerge: () => void;
  merging: boolean;
}

export function DuplicateGroupCard({
  group,
  groupKey,
  selectedId,
  deleteFiles,
  onSelect,
  onMerge,
  merging,
}: DuplicateGroupCardProps) {
  const keepId = selectedId ?? group.scenes[0]?.id;

  return (
    <li className="rounded-md border border-[var(--color-border)] p-3 space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="font-medium text-sm">
          {group.match_type}
          {group.max_distance != null ? ` · max distance ${group.max_distance}` : ""}
          {" · "}
          {group.scenes.length} scenes
        </p>
        <Button
          size="sm"
          disabled={!keepId || merging || group.scenes.length < 2}
          onClick={onMerge}
        >
          Keep selected
        </Button>
      </div>

      <ul className="space-y-2">
        {group.scenes.map((scene) => (
          <ScenePickRow
            key={scene.id}
            scene={scene}
            groupKey={groupKey}
            checked={keepId === scene.id}
            onSelect={() => onSelect(scene.id)}
          />
        ))}
      </ul>

      {deleteFiles && (
        <p className="text-xs text-amber-500">Removed scene files will be deleted from disk.</p>
      )}
    </li>
  );
}

function ScenePickRow({
  scene,
  groupKey,
  checked,
  onSelect,
}: {
  scene: Scene;
  groupKey: string;
  checked: boolean;
  onSelect: () => void;
}) {
  return (
    <label className="flex cursor-pointer items-start gap-2 rounded-md border border-transparent px-2 py-1.5 hover:border-[var(--color-border)]">
      <input
        type="radio"
        name={`dup-${groupKey}`}
        checked={checked}
        onChange={onSelect}
        className="mt-1"
      />
      <span className="min-w-0 flex-1 text-xs">
        <span className="block font-medium text-sm">{scene.title}</span>
        {scene.path && (
          <span className="block truncate text-[var(--color-muted-foreground)]">{scene.path}</span>
        )}
      </span>
    </label>
  );
}
