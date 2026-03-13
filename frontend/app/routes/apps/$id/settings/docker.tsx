import { useState } from "react";
import { useOutletContext } from "react-router";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { api } from "@/lib/api";
import type { App, UpdateAppRequest } from "@/types/api";

export function meta() {
  return [
    { title: "Docker Options - App Settings - Rivetr" },
    { name: "description", content: "Configure custom Docker container run options" },
  ];
}

function parseJsonArray(json: string | null | undefined): string[] {
  if (!json) return [];
  try {
    const parsed = JSON.parse(json);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

export default function AppSettingsDocker() {
  const { app } = useOutletContext<{ app: App }>();
  const queryClient = useQueryClient();
  const [isSubmitting, setIsSubmitting] = useState(false);

  const [privileged, setPrivileged] = useState(app.privileged || false);
  const [initProcess, setInitProcess] = useState(app.init_process || false);
  const [capAdd, setCapAdd] = useState(
    parseJsonArray(app.cap_add).join(", ")
  );
  const [devices, setDevices] = useState(
    parseJsonArray(app.devices).join("\n")
  );
  const [shmSize, setShmSize] = useState(app.shm_size || "");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    try {
      // Parse cap_add: comma-separated capabilities
      const parsedCapAdd = capAdd
        .split(",")
        .map((s) => s.trim().toUpperCase())
        .filter((s) => s.length > 0);

      // Parse devices: one per line
      const parsedDevices = devices
        .split("\n")
        .map((s) => s.trim())
        .filter((s) => s.length > 0);

      const updates: UpdateAppRequest = {
        privileged,
        init_process: initProcess,
        cap_add: parsedCapAdd,
        devices: parsedDevices,
        shm_size: shmSize.trim() || undefined,
      };

      await api.updateApp(app.id, updates);
      toast.success("Docker options saved");
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Update failed");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Advanced Docker Options</CardTitle>
          <CardDescription>
            Configure custom Docker container run options. Changes take effect on the next
            deployment. Use these settings with caution — privileged mode and capability
            additions can reduce container isolation.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-6">
            {/* Privileged Mode */}
            <div className="flex items-center justify-between rounded-lg border p-4">
              <div className="space-y-0.5">
                <Label htmlFor="privileged" className="text-base font-medium">
                  Privileged Mode
                </Label>
                <p className="text-sm text-muted-foreground">
                  Run the container with elevated privileges. Required for some system-level
                  operations (e.g., Docker-in-Docker). Use with caution.
                </p>
              </div>
              <Switch
                id="privileged"
                checked={privileged}
                onCheckedChange={setPrivileged}
              />
            </div>

            {/* Init Process */}
            <div className="flex items-center justify-between rounded-lg border p-4">
              <div className="space-y-0.5">
                <Label htmlFor="init_process" className="text-base font-medium">
                  Init Process (tini)
                </Label>
                <p className="text-sm text-muted-foreground">
                  Run tini as PID 1 inside the container. Helps with signal forwarding and
                  zombie process reaping.
                </p>
              </div>
              <Switch
                id="init_process"
                checked={initProcess}
                onCheckedChange={setInitProcess}
              />
            </div>

            {/* Cap Add */}
            <div className="space-y-2">
              <Label htmlFor="cap_add">Add Capabilities</Label>
              <Textarea
                id="cap_add"
                placeholder="NET_ADMIN, SYS_PTRACE"
                value={capAdd}
                onChange={(e) => setCapAdd(e.target.value)}
                rows={2}
              />
              <p className="text-xs text-muted-foreground">
                Comma-separated list of Linux capabilities to add (e.g.{" "}
                <code className="font-mono">NET_ADMIN, SYS_PTRACE</code>).
              </p>
            </div>

            {/* Devices */}
            <div className="space-y-2">
              <Label htmlFor="devices">Device Mappings</Label>
              <Textarea
                id="devices"
                placeholder="/dev/snd:/dev/snd"
                value={devices}
                onChange={(e) => setDevices(e.target.value)}
                rows={3}
              />
              <p className="text-xs text-muted-foreground">
                One device mapping per line in the format{" "}
                <code className="font-mono">host_path:container_path</code> (e.g.{" "}
                <code className="font-mono">/dev/snd:/dev/snd</code>).
              </p>
            </div>

            {/* SHM Size */}
            <div className="space-y-2">
              <Label htmlFor="shm_size">Shared Memory Size</Label>
              <Input
                id="shm_size"
                placeholder="128m"
                value={shmSize}
                onChange={(e) => setShmSize(e.target.value)}
              />
              <p className="text-xs text-muted-foreground">
                Size of <code className="font-mono">/dev/shm</code> (e.g.{" "}
                <code className="font-mono">128m</code>,{" "}
                <code className="font-mono">1g</code>). Leave empty for the Docker default.
              </p>
            </div>

            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? "Saving..." : "Save Docker Options"}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
