import { useState, useEffect } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Eye, Plus, X } from "lucide-react";
import { api } from "@/lib/api";
import type { App, UpdateAppRequest } from "@/types/api";

interface WatchPathsCardProps {
  app: App;
  token?: string;
}

function parseWatchPaths(json: string | null): string[] {
  if (!json) return [];
  try {
    const parsed = JSON.parse(json);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

export function WatchPathsCard({ app, token }: WatchPathsCardProps) {
  const queryClient = useQueryClient();
  const [paths, setPaths] = useState<string[]>(
    parseWatchPaths(app.watch_paths),
  );
  const [newPath, setNewPath] = useState("");
  const [hasChanges, setHasChanges] = useState(false);

  useEffect(() => {
    setPaths(parseWatchPaths(app.watch_paths));
    setHasChanges(false);
  }, [app.watch_paths]);

  useEffect(() => {
    const current = JSON.stringify(paths);
    const original = JSON.stringify(parseWatchPaths(app.watch_paths));
    setHasChanges(current !== original);
  }, [paths, app.watch_paths]);

  const updateMutation = useMutation({
    mutationFn: (data: UpdateAppRequest) => api.updateApp(app.id, data, token),
    onSuccess: () => {
      toast.success("Watch paths updated");
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
      setHasChanges(false);
    },
    onError: (error: Error) => {
      toast.error(`Failed to update watch paths: ${error.message}`);
    },
  });

  const addPath = () => {
    const trimmed = newPath.trim();
    if (!trimmed) {
      toast.error("Path pattern cannot be empty");
      return;
    }
    if (paths.includes(trimmed)) {
      toast.error("This pattern is already added");
      return;
    }
    setPaths([...paths, trimmed]);
    setNewPath("");
  };

  const removePath = (index: number) => {
    setPaths(paths.filter((_, i) => i !== index));
  };

  const handleSave = () => {
    const watchPathsValue =
      paths.length > 0 ? JSON.stringify(paths) : undefined;
    updateMutation.mutate({
      watch_paths: watchPathsValue,
    });
  };

  const handleReset = () => {
    setPaths(parseWatchPaths(app.watch_paths));
    setNewPath("");
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Eye className="h-5 w-5" />
          Watch Paths
        </CardTitle>
        <CardDescription>
          Configure file path patterns to control when webhook pushes trigger a
          deployment. Only pushes that modify files matching these patterns will
          start a new deployment. Leave empty to deploy on every push.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {paths.length > 0 && (
          <div className="flex flex-wrap gap-2">
            {paths.map((path, index) => (
              <Badge
                key={index}
                variant="secondary"
                className="flex items-center gap-1 px-3 py-1.5 text-sm font-mono"
              >
                {path}
                <button
                  onClick={() => removePath(index)}
                  className="ml-1 rounded-full hover:bg-muted-foreground/20 p-0.5"
                  aria-label={`Remove ${path}`}
                >
                  <X className="h-3 w-3" />
                </button>
              </Badge>
            ))}
          </div>
        )}

        {paths.length === 0 && (
          <div className="text-sm text-muted-foreground py-3 text-center border rounded-md">
            No watch paths configured. All pushes will trigger deployments.
          </div>
        )}

        <div className="flex gap-2">
          <Input
            placeholder="e.g., src/**, package.json, Dockerfile"
            value={newPath}
            onChange={(e) => setNewPath(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                addPath();
              }
            }}
            className="font-mono"
          />
          <Button
            variant="outline"
            size="sm"
            onClick={addPath}
            disabled={!newPath.trim()}
          >
            <Plus className="h-4 w-4 mr-1" />
            Add
          </Button>
        </div>

        <p className="text-xs text-muted-foreground">
          Use glob patterns like <code className="bg-muted px-1 rounded">src/**</code>,{" "}
          <code className="bg-muted px-1 rounded">*.json</code>, or{" "}
          <code className="bg-muted px-1 rounded">Dockerfile</code>. Directory
          patterns ending with <code className="bg-muted px-1 rounded">/</code>{" "}
          will match all files within that directory.
        </p>

        {hasChanges && (
          <div className="flex gap-2 pt-2 border-t">
            <Button
              onClick={handleSave}
              disabled={updateMutation.isPending}
            >
              {updateMutation.isPending ? "Saving..." : "Save Changes"}
            </Button>
            <Button variant="outline" onClick={handleReset}>
              Reset
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
