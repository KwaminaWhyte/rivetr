import { useState, useEffect } from "react";
import { toast } from "sonner";
import { Plus, Trash2, GripVertical, Terminal } from "lucide-react";
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
import { api } from "@/lib/api";
import type { App, UpdateAppRequest } from "@/types/api";

interface DeploymentCommandsCardProps {
  app: App;
  token?: string;
  onSave?: () => void;
}

export function DeploymentCommandsCard({
  app,
  token,
  onSave,
}: DeploymentCommandsCardProps) {
  const [preDeployCommands, setPreDeployCommands] = useState<string[]>([]);
  const [postDeployCommands, setPostDeployCommands] = useState<string[]>([]);
  const [isSaving, setIsSaving] = useState(false);

  // Parse JSON from app on mount or when app changes
  useEffect(() => {
    try {
      const preCmds = app.pre_deploy_commands
        ? JSON.parse(app.pre_deploy_commands)
        : [];
      setPreDeployCommands(Array.isArray(preCmds) ? preCmds : []);
    } catch {
      setPreDeployCommands([]);
    }

    try {
      const postCmds = app.post_deploy_commands
        ? JSON.parse(app.post_deploy_commands)
        : [];
      setPostDeployCommands(Array.isArray(postCmds) ? postCmds : []);
    } catch {
      setPostDeployCommands([]);
    }
  }, [app.pre_deploy_commands, app.post_deploy_commands]);

  const handleAddPreCommand = () => {
    setPreDeployCommands([...preDeployCommands, ""]);
  };

  const handleAddPostCommand = () => {
    setPostDeployCommands([...postDeployCommands, ""]);
  };

  const handleUpdatePreCommand = (index: number, value: string) => {
    const updated = [...preDeployCommands];
    updated[index] = value;
    setPreDeployCommands(updated);
  };

  const handleUpdatePostCommand = (index: number, value: string) => {
    const updated = [...postDeployCommands];
    updated[index] = value;
    setPostDeployCommands(updated);
  };

  const handleRemovePreCommand = (index: number) => {
    setPreDeployCommands(preDeployCommands.filter((_, i) => i !== index));
  };

  const handleRemovePostCommand = (index: number) => {
    setPostDeployCommands(postDeployCommands.filter((_, i) => i !== index));
  };

  const handleSave = async () => {
    setIsSaving(true);
    try {
      // Filter out empty commands
      const filteredPre = preDeployCommands.filter((cmd) => cmd.trim() !== "");
      const filteredPost = postDeployCommands.filter(
        (cmd) => cmd.trim() !== "",
      );

      const updates: UpdateAppRequest = {
        pre_deploy_commands: filteredPre.length > 0 ? filteredPre : undefined,
        post_deploy_commands:
          filteredPost.length > 0 ? filteredPost : undefined,
      };

      await api.updateApp(app.id, updates, token);
      toast.success("Deployment commands saved");
      onSave?.();
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to save commands",
      );
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Terminal className="h-5 w-5" />
          Deployment Commands
        </CardTitle>
        <CardDescription>
          Configure commands to run during the deployment pipeline. Pre-deploy
          commands run before the health check, and post-deploy commands run
          after the container is healthy.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Pre-Deploy Commands */}
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div>
              <Label className="text-base">Pre-Deploy Commands</Label>
              <p className="text-sm text-muted-foreground">
                Run after container starts, before health check
              </p>
            </div>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={handleAddPreCommand}
            >
              <Plus className="h-4 w-4 mr-1" />
              Add
            </Button>
          </div>

          {preDeployCommands.length === 0 ? (
            <p className="text-sm text-muted-foreground italic py-2">
              No pre-deploy commands configured
            </p>
          ) : (
            <div className="space-y-2">
              {preDeployCommands.map((cmd, index) => (
                <div key={index} className="flex items-center gap-2">
                  <div className="flex items-center text-muted-foreground">
                    <GripVertical className="h-4 w-4" />
                    <span className="w-6 text-xs text-center">
                      {index + 1}.
                    </span>
                  </div>
                  <Input
                    value={cmd}
                    onChange={(e) =>
                      handleUpdatePreCommand(index, e.target.value)
                    }
                    placeholder="e.g., npm run db:migrate"
                    className="font-mono text-sm"
                  />
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    onClick={() => handleRemovePreCommand(index)}
                    className="text-muted-foreground hover:text-destructive"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Post-Deploy Commands */}
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div>
              <Label className="text-base">Post-Deploy Commands</Label>
              <p className="text-sm text-muted-foreground">
                Run after health check passes
              </p>
            </div>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={handleAddPostCommand}
            >
              <Plus className="h-4 w-4 mr-1" />
              Add
            </Button>
          </div>

          {postDeployCommands.length === 0 ? (
            <p className="text-sm text-muted-foreground italic py-2">
              No post-deploy commands configured
            </p>
          ) : (
            <div className="space-y-2">
              {postDeployCommands.map((cmd, index) => (
                <div key={index} className="flex items-center gap-2">
                  <div className="flex items-center text-muted-foreground">
                    <GripVertical className="h-4 w-4" />
                    <span className="w-6 text-xs text-center">
                      {index + 1}.
                    </span>
                  </div>
                  <Input
                    value={cmd}
                    onChange={(e) =>
                      handleUpdatePostCommand(index, e.target.value)
                    }
                    placeholder="e.g., npm run cache:warm"
                    className="font-mono text-sm"
                  />
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    onClick={() => handleRemovePostCommand(index)}
                    className="text-muted-foreground hover:text-destructive"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Save Button */}
        <Button
          onClick={handleSave}
          disabled={isSaving}
          className="w-full sm:w-auto"
        >
          {isSaving ? "Saving..." : "Save Commands"}
        </Button>
      </CardContent>
    </Card>
  );
}
