import { useEffect, useRef } from "react";
import { getCapabilities } from "@/lib/runtime";
import { isVideoScene } from "@/lib/mediaUrl";
import type { Scene } from "@/lib/types";

export interface SceneContextMenuState {
  scene: Scene;
  x: number;
  y: number;
}

interface SceneContextMenuProps {
  menu: SceneContextMenuState | null;
  onClose: () => void;
  onEdit: (scene: Scene) => void;
  onDetails: (scene: Scene) => void;
  onPlay?: (scene: Scene) => void;
  onOpenExplorer?: (scene: Scene) => void;
  onOpenDefault?: (scene: Scene) => void;
  onRenameFile?: (scene: Scene) => void;
  onDelete?: (scene: Scene) => void;
}

export function SceneContextMenu({
  menu,
  onClose,
  onEdit,
  onDetails,
  onPlay,
  onOpenExplorer,
  onOpenDefault,
  onRenameFile,
  onDelete,
}: SceneContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null);
  const caps = getCapabilities();

  useEffect(() => {
    if (!menu) return;
    function handleClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    }
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("mousedown", handleClick);
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("mousedown", handleClick);
      document.removeEventListener("keydown", handleKey);
    };
  }, [menu, onClose]);

  if (!menu) return null;

  const items: { label: string; action: () => void; show?: boolean; danger?: boolean }[] = [
    {
      label: "Edit metadata",
      action: () => {
        onEdit(menu.scene);
        onClose();
      },
    },
    {
      label: "Rename file to match title",
      action: () => {
        onRenameFile?.(menu.scene);
        onClose();
      },
      show: Boolean(menu.scene.path) && Boolean(onRenameFile),
    },
    {
      label: "Play",
      action: () => {
        onPlay?.(menu.scene);
        onClose();
      },
      show: Boolean(menu.scene.path) && isVideoScene(menu.scene) && Boolean(onPlay),
    },
    {
      label: "Details",
      action: () => {
        onDetails(menu.scene);
        onClose();
      },
    },
    {
      label: "Open in Explorer",
      action: () => {
        onOpenExplorer?.(menu.scene);
        onClose();
      },
      show: caps.localIpc && Boolean(menu.scene.path) && Boolean(onOpenExplorer),
    },
    {
      label: "Open with default app",
      action: () => {
        onOpenDefault?.(menu.scene);
        onClose();
      },
      show: caps.localIpc && Boolean(menu.scene.path) && Boolean(onOpenDefault),
    },
    {
      label: "Delete…",
      action: () => {
        onDelete?.(menu.scene);
        onClose();
      },
      show: Boolean(onDelete),
      danger: true,
    },
  ];

  return (
    <div
      ref={ref}
      className="fixed z-50 min-w-[180px] rounded-md border border-[var(--color-border)] bg-[var(--color-card)] py-1 shadow-lg"
      style={{ left: menu.x, top: menu.y }}
    >
      {items
        .filter((item) => item.show !== false)
        .map((item) => (
          <button
            key={item.label}
            type="button"
            className={`block w-full px-3 py-1.5 text-left text-sm hover:bg-[var(--color-muted)] ${
              item.danger ? "text-red-400" : ""
            }`}
            onClick={item.action}
          >
            {item.label}
          </button>
        ))}
    </div>
  );
}
