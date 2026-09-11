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
import type { CollectionType } from "@/lib/types";
import { Folder, ListTodo } from "lucide-react";

interface CollectionFormDialogProps {
  open: boolean;
  onClose: () => void;
  onSubmit: (data: { name: string; type: CollectionType; description?: string }) => void;
}

export function CollectionFormDialog({ open, onClose, onSubmit }: CollectionFormDialogProps) {
  const [name, setName] = useState("");
  const [type, setType] = useState<CollectionType>("collection");
  const [description, setDescription] = useState("");

  const handleSubmit = () => {
    if (!name.trim()) return;
    onSubmit({ name: name.trim(), type, description: description.trim() || undefined });
    setName("");
    setDescription("");
    setType("collection");
    onClose();
  };

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent>
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
          </div>
          <Input
            placeholder="Description (optional)"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
          />
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
