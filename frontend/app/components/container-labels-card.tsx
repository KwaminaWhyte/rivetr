import { useState, useEffect } from "react";
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
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Plus, Trash2, Tag, Wand2 } from "lucide-react";
import type { App, UpdateAppRequest } from "@/types/api";

interface ContainerLabelsCardProps {
  app: App;
  onSave: (updates: UpdateAppRequest) => Promise<void>;
  isSaving?: boolean;
}

// Helper to parse container_labels JSON from app
function parseLabels(json: string | null): Record<string, string> {
  if (!json) return {};
  try {
    return JSON.parse(json);
  } catch {
    return {};
  }
}

// Label presets for common reverse proxies
interface LabelPreset {
  name: string;
  description: string;
  labels: Record<string, string>;
}

function getTraefikPreset(appName: string, domain: string): LabelPreset {
  return {
    name: "Traefik",
    description: "Labels for Traefik reverse proxy",
    labels: {
      "traefik.enable": "true",
      [`traefik.http.routers.${appName}.rule`]: `Host(\`${domain || "example.com"}\`)`,
      [`traefik.http.routers.${appName}.entrypoints`]: "websecure",
      [`traefik.http.routers.${appName}.tls.certresolver`]: "letsencrypt",
      [`traefik.http.services.${appName}.loadbalancer.server.port`]: "80",
    },
  };
}

function getCaddyPreset(appName: string, domain: string): LabelPreset {
  return {
    name: "Caddy",
    description: "Labels for Caddy reverse proxy (caddy-docker-proxy)",
    labels: {
      "caddy": domain || "example.com",
      "caddy.reverse_proxy": `{{upstreams 80}}`,
      "caddy.tls": "",
    },
  };
}

function getDockerComposePreset(appName: string): LabelPreset {
  return {
    name: "Docker Compose",
    description: "Common Docker Compose labels",
    labels: {
      "com.docker.compose.project": appName,
      "com.docker.compose.service": appName,
    },
  };
}

export function ContainerLabelsCard({
  app,
  onSave,
  isSaving = false,
}: ContainerLabelsCardProps) {
  // Parse current labels from app
  const [labels, setLabels] = useState<Record<string, string>>(
    parseLabels(app.container_labels)
  );

  // Form state for adding new labels
  const [newKey, setNewKey] = useState("");
  const [newValue, setNewValue] = useState("");

  // Track if there are unsaved changes
  const [hasChanges, setHasChanges] = useState(false);

  // Update state when app changes
  useEffect(() => {
    setLabels(parseLabels(app.container_labels));
    setHasChanges(false);
  }, [app.container_labels]);

  // Add a new label
  const addLabel = () => {
    if (!newKey.trim()) {
      toast.error("Label key cannot be empty");
      return;
    }
    if (labels[newKey.trim()]) {
      toast.error("Label key already exists");
      return;
    }

    setLabels({ ...labels, [newKey.trim()]: newValue.trim() });
    setNewKey("");
    setNewValue("");
    setHasChanges(true);
  };

  // Remove a label
  const removeLabel = (key: string) => {
    const newLabels = { ...labels };
    delete newLabels[key];
    setLabels(newLabels);
    setHasChanges(true);
  };

  // Update an existing label value
  const updateLabelValue = (key: string, value: string) => {
    setLabels({ ...labels, [key]: value });
    setHasChanges(true);
  };

  // Apply a preset
  const applyPreset = (presetType: string) => {
    const appName = app.name.toLowerCase().replace(/[^a-z0-9-]/g, "-");
    const domain = app.domain || app.auto_subdomain || "example.com";

    let preset: LabelPreset;
    switch (presetType) {
      case "traefik":
        preset = getTraefikPreset(appName, domain);
        break;
      case "caddy":
        preset = getCaddyPreset(appName, domain);
        break;
      case "docker-compose":
        preset = getDockerComposePreset(appName);
        break;
      default:
        return;
    }

    // Merge preset labels with existing labels
    setLabels({ ...labels, ...preset.labels });
    setHasChanges(true);
    toast.success(`Applied ${preset.name} preset`);
  };

  // Save handler
  const handleSave = async () => {
    try {
      await onSave({
        container_labels: labels,
      });
      setHasChanges(false);
      toast.success("Container labels saved");
    } catch (error) {
      toast.error(
        `Failed to save: ${error instanceof Error ? error.message : "Unknown error"}`
      );
    }
  };

  // Clear all labels
  const clearAllLabels = () => {
    setLabels({});
    setHasChanges(true);
  };

  const labelEntries = Object.entries(labels);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Tag className="h-5 w-5" />
          Container Labels
        </CardTitle>
        <CardDescription>
          Add custom labels to your container. Labels are key-value pairs that can be
          used by reverse proxies (Traefik, Caddy), monitoring tools, or for organization.
          Changes will take effect on the next deployment.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Preset Templates */}
        <div className="space-y-3">
          <div className="flex items-center gap-2">
            <Wand2 className="h-4 w-4 text-muted-foreground" />
            <Label className="text-sm font-medium">Label Presets</Label>
          </div>
          <p className="text-xs text-muted-foreground">
            Quickly add common label configurations for reverse proxies.
          </p>
          <div className="flex flex-wrap gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => applyPreset("traefik")}
            >
              Traefik
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => applyPreset("caddy")}
            >
              Caddy
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => applyPreset("docker-compose")}
            >
              Docker Compose
            </Button>
          </div>
        </div>

        {/* Current Labels */}
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Tag className="h-4 w-4 text-muted-foreground" />
              <Label className="text-sm font-medium">
                Labels ({labelEntries.length})
              </Label>
            </div>
            {labelEntries.length > 0 && (
              <Button
                variant="ghost"
                size="sm"
                onClick={clearAllLabels}
                className="text-red-500 hover:text-red-600"
              >
                Clear All
              </Button>
            )}
          </div>

          {labelEntries.length > 0 && (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[40%]">Key</TableHead>
                  <TableHead>Value</TableHead>
                  <TableHead className="w-[60px]"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {labelEntries.map(([key, value]) => (
                  <TableRow key={key}>
                    <TableCell className="font-mono text-sm break-all">
                      {key}
                    </TableCell>
                    <TableCell>
                      <Input
                        value={value}
                        onChange={(e) => updateLabelValue(key, e.target.value)}
                        className="font-mono text-sm h-8"
                      />
                    </TableCell>
                    <TableCell>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => removeLabel(key)}
                        className="h-7 w-7 p-0 text-red-500 hover:text-red-600"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}

          {labelEntries.length === 0 && (
            <div className="text-sm text-muted-foreground py-4 text-center border rounded-md">
              No labels configured. Add labels below or use a preset above.
            </div>
          )}
        </div>

        {/* Add New Label */}
        <div className="space-y-3">
          <Label className="text-sm font-medium">Add New Label</Label>
          <div className="flex gap-2 items-end">
            <div className="flex-1 space-y-1">
              <Label className="text-xs">Key</Label>
              <Input
                placeholder="com.example.label"
                value={newKey}
                onChange={(e) => setNewKey(e.target.value)}
                className="font-mono"
                onKeyDown={(e) => e.key === "Enter" && newKey && addLabel()}
              />
            </div>
            <div className="flex-1 space-y-1">
              <Label className="text-xs">Value</Label>
              <Input
                placeholder="value"
                value={newValue}
                onChange={(e) => setNewValue(e.target.value)}
                className="font-mono"
                onKeyDown={(e) => e.key === "Enter" && newKey && addLabel()}
              />
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={addLabel}
              disabled={!newKey.trim()}
            >
              <Plus className="h-4 w-4 mr-1" />
              Add
            </Button>
          </div>
        </div>

        {/* Save Button */}
        {hasChanges && (
          <div className="flex justify-end pt-4 border-t">
            <Button onClick={handleSave} disabled={isSaving}>
              {isSaving ? "Saving..." : "Save Container Labels"}
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
