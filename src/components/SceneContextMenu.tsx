import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { getCapabilities } from "@/lib/runtime";
import { isMobileDevice } from "@/lib/tauri";
import { isVideoScene } from "@/lib/mediaUrl";
import type { Scene } from "@/lib/types";
import { ShortcutBadge } from "@/components/ui/shortcut-badge";

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
  onRegenThumb?: (scene: Scene) => void;
  onDelete?: (scene: Scene) => void;
  /** #26: toggle watched state; `watched` is the scene's current state. */
  onMarkWatched?: (scene: Scene, watched: boolean) => void;
  watched?: boolean;
  /** #2/#6: open collection picker to add/remove this scene. */
  onAddToCollection?: (scene: Scene) => void;
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
  onRegenThumb,
  onDelete,
  onMarkWatched,
  watched,
  onAddToCollection,
}: SceneContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null);
  const caps = getCapabilities();
  // Phones get a bottom sheet (large targets, above the system nav bar)
  // instead of a desktop right-click popup at the touch point.
  const sheet = isMobileDevice();
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null);

  // Desktop: keep the popup inside the window (flip up/left near edges).
  useLayoutEffect(() => {
    if (!menu || sheet || !ref.current) return;
    const { width, height } = ref.current.getBoundingClientRect();
    const margin = 8;
    const left = Math.max(margin, Math.min(menu.x, window.innerWidth - width - margin));
    const top =
      menu.y + height + margin > window.innerHeight ? Math.max(margin, menu.y - height) : menu.y;
    setPos({ left, top });
  }, [menu, sheet]);

  useEffect(() => {
    if (!menu) return;
    function handleClick(e: Event) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    }
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("mousedown", handleClick);
    document.addEventListener("touchstart", handleClick, { passive: true });
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("mousedown", handleClick);
      document.removeEventListener("touchstart", handleClick);
      document.removeEventListener("keydown", handleKey);
    };
  }, [menu, onClose]);

  if (!menu) return null;

  const items: {
    label: string;
    action: () => void;
    show?: boolean;
    danger?: boolean;
    shortcut?: string;
  }[] = [
    {
      label: "Play",
      action: () => {
        onPlay?.(menu.scene);
        onClose();
      },
      show: Boolean(menu.scene.path) && isVideoScene(menu.scene) && Boolean(onPlay),
    },
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
      label: "Details",
      action: () => {
        onDetails(menu.scene);
        onClose();
      },
    },
    {
      label: watched ? "Mark as unwatched" : "Mark as watched",
      action: () => {
        onMarkWatched?.(menu.scene, !watched);
        onClose();
      },
      shortcut: "w",
      show: Boolean(onMarkWatched),
    },
    {
      label: "Add to collection",
      action: () => {
        onAddToCollection?.(menu.scene);
        onClose();
      },
      show: Boolean(onAddToCollection),
    },
    {
      label: "Regenerate thumbnail",
      action: () => {
        onRegenThumb?.(menu.scene);
        onClose();
      },
      show: Boolean(menu.scene.path) && isVideoScene(menu.scene) && Boolean(onRegenThumb),
    },
    {
      label: "Open in Explorer",
      action: () => {
        onOpenExplorer?.(menu.scene);
        onClose();
      },
      // Revealing a file in a folder window is a desktop concept.
      show: !sheet && caps.localIpc && Boolean(menu.scene.path) && Boolean(onOpenExplorer),
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

  const visible = items.filter((item) => item.show !== false);

  if (sheet) {
    return (
      <div className="fixed inset-0 z-[110] flex items-end bg-black/50" role="presentation">
        <div
          ref={ref}
          role="menu"
          className="max-h-[80dvh] w-full overflow-y-auto rounded-t-2xl border-t border-[var(--color-border)] bg-[var(--color-card)] px-2 pt-2 pb-[max(1rem,env(safe-area-inset-bottom))] shadow-lg"
        >
          <div className="mx-auto mb-2 h-1 w-10 rounded-full bg-[var(--color-muted)]" />
          <p className="truncate px-3 pb-2 text-sm font-medium">{menu.scene.title}</p>
          {visible.map((item) => (
            <button
              key={item.label}
              type="button"
              role="menuitem"
              className={`flex w-full items-center rounded-md px-3 py-3 text-left text-base active:bg-[var(--color-muted)] ${
                item.danger ? "text-red-400" : ""
              }`}
              onClick={item.action}
            >
              {item.label}
            </button>
          ))}
        </div>
      </div>
    );
  }

  return (
    <div
      ref={ref}
      role="menu"
      className="fixed z-[110] min-w-[180px] rounded-md border border-[var(--color-border)] bg-[var(--color-card)] py-1 shadow-lg"
      style={{
        left: pos?.left ?? menu.x,
        top: pos?.top ?? menu.y,
        visibility: pos ? "visible" : "hidden",
      }}
    >
      {visible.map((item) => (
        <button
          key={item.label}
          type="button"
          role="menuitem"
          className={`flex w-full items-center justify-between px-3 py-1.5 text-left text-sm hover:bg-[var(--color-muted)] ${
            item.danger ? "text-red-400" : ""
          }`}
          onClick={item.action}
        >
          <span>{item.label}</span>
          {item.shortcut && <ShortcutBadge keys={item.shortcut} />}
        </button>
      ))}
    </div>
  );
}
