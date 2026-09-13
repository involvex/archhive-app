import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { CollectionType, SceneFilter } from "@/lib/types";
import { Folder, ListTodo, Zap } from "lucide-react";
import { FilterBuilder } from "@/components/FilterBuilder";

interface CollectionFormDialogProps {
  open: boolean;
  onClose: () => void;
  onSubmit: (data: {
    name: string;
    type: CollectionType;
    description?: string;
    filter?: SceneFilter;
  }) => void;
}

export function CollectionFormDialog({ open, onClose, onSubmit }: CollectionFormDialogProps) {
  const [name, setName] = useState("");
  const [type, setType] = useState<CollectionType>("collection");
  const [description, setDescription] = useState("");
  const [filter, setFilter] = useState<SceneFilter>({});
  const [performerInput, setPerformerInput] = useState("");
  const [tagInput, setTagInput] = useState("");

  const isSmart = type === "smart";

  const handleSubmit = () => {
    if (!name.trim()) return;
    onSubmit({
      name: name.trim(),
      type,
      description: description.trim() || undefined,
      filter:
        isSmart &&
        (Object.keys(filter).length > 0 ||
          filter.performer_names?.length ||
          filter.tag_names?.length)
          ? filter
          : undefined,
    });
    setName("");
    setDescription("");
    setFilter({});
    setPerformerInput("");
    setTagInput("");
    setType("collection");
    onClose();
  };

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>New group</DialogTitle>
        </DialogHeader>
        <div className="space-y-3 mt-2">
          <Input
            placeholder="Name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            autoFocus
          />
          <div className="flex gap-2">
            <Button
              variant={type === "collection" ? "default" : "outline"}
              size="sm"
              onClick={() => setType("collection")}
            >
              <Folder className="h-4 w-4 mr-1" />
              Collection
            </Button>
            <Button
              variant={type === "watchlist" ? "default" : "outline"}
              size="sm"
              onClick={() => setType("watchlist")}
            >
              <ListTodo className="h-4 w-4 mr-1" />
              Watchlist
            </Button>
            <Button
              variant={type === "smart" ? "default" : "outline"}
              size="sm"
              onClick={() => setType("smart")}
            >
              <Zap className="h-4 w-4 mr-1" />
              Smart
            </Button>
          </div>
          <textarea
            placeholder="Description (optional)"
            value={description}
            onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) => setDescription(e.target.value)}
            rows={2}
            className="w-full rounded-md border border-[var(--color-border)] bg-[var(--color-background)] px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-[var(--color-primary)]"
          />
          {isSmart && (
            <div className="space-y-2 pt-2 border-t border-[var(--color-border)]">
              <p className="text-xs text-[var(--color-muted-foreground)]">
                Smart collections automatically include scenes matching the filter below. Scenes are
                re-evaluated each time the collection is opened.
              </p>
              <FilterBuilder
                filter={filter}
                onChange={setFilter}
                performerInput={performerInput}
                onPerformerInputChange={setPerformerInput}
                tagInput={tagInput}
                onTagInputChange={setTagInput}
                showDurationInputs={true}
                showFileSizeInput={true}
                showPerformerTagInputs={true}
                showRatingFilters={true}
              />
            </div>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" size="sm" onClick={onClose}>
            Cancel
          </Button>
          <Button size="sm" onClick={handleSubmit}>
            Create
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
