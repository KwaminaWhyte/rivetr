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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
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
  const [isApplyingLimits, setIsApplyingLimits] = useState(false);

  const [memoryLimit, setMemoryLimit] = useState(app.memory_limit || "");
  const [cpuLimit, setCpuLimit] = useState(app.cpu_limit || "");
  const [restartPolicy, setRestartPolicy] = useState(app.restart_policy || "unless-stopped");
  const [privileged, setPrivileged] = useState(app.privileged || false);
  const [initProcess, setInitProcess] = useState(app.init_process || false);
  const [capAdd, setCapAdd] = useState(parseJsonArray(app.cap_add).join(", "));
  const [capDrop, setCapDrop] = useState(parseJsonArray(app.docker_cap_drop).join(", "));
  const [devices, setDevices] = useState(parseJsonArray(app.devices).join("\n"));
  const [shmSize, setShmSize] = useState(app.shm_size || "");
  const [gpus, setGpus] = useState(app.docker_gpus || "");
  const [ulimits, setUlimits] = useState(parseJsonArray(app.docker_ulimits).join("\n"));
  const [securityOpt, setSecurityOpt] = useState(
    parseJsonArray(app.docker_security_opt).join("\n")
  );

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    try {
      const parsedCapAdd = capAdd
        .split(",")
        .map((s) => s.trim().toUpperCase())
        .filter((s) => s.length > 0);

      const parsedCapDrop = capDrop
        .split(",")
        .map((s) => s.trim().toUpperCase())
        .filter((s) => s.length > 0);

      const parsedDevices = devices
        .split("\n")
        .map((s) => s.trim())
        .filter((s) => s.length > 0);

      const parsedUlimits = ulimits
        .split("\n")
        .map((s) => s.trim())
        .filter((s) => s.length > 0);

      const parsedSecurityOpt = securityOpt
        .split("\n")
        .map((s) => s.trim())
        .filter((s) => s.length > 0);

      const updates: UpdateAppRequest = {
        memory_limit: memoryLimit.trim() || undefined,
        cpu_limit: cpuLimit.trim() || undefined,
        restart_policy: restartPolicy,
        privileged,
        init_process: initProcess,
        cap_add: parsedCapAdd,
        docker_cap_drop: parsedCapDrop,
        devices: parsedDevices,
        shm_size: shmSize.trim() || undefined,
        docker_gpus: gpus.trim() || "",
        docker_ulimits: parsedUlimits,
        docker_security_opt: parsedSecurityOpt,
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

  const handleApplyLimits = async () => {
    setIsApplyingLimits(true);
    try {
      // Save limits first, then apply live
      await api.updateApp(app.id, {
        memory_limit: memoryLimit.trim() || undefined,
        cpu_limit: cpuLimit.trim() || undefined,
      });
      await api.applyResourceLimits(app.id);
      toast.success("Resource limits applied to running container");
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to apply limits");
    } finally {
      setIsApplyingLimits(false);
    }
  };

  return (
    <div className="space-y-6">
      {/* Resource Limits */}
      <Card>
        <CardHeader>
          <CardTitle>Resource Limits</CardTitle>
          <CardDescription>
            Set CPU and memory limits to prevent this container from starving other apps.
            Use "Apply Now" to enforce limits on the running container immediately without a redeploy.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="memory_limit">Memory Limit</Label>
              <Input
                id="memory_limit"
                placeholder="512m"
                value={memoryLimit}
                onChange={(e) => setMemoryLimit(e.target.value)}
              />
              <p className="text-xs text-muted-foreground">
                e.g. <code className="font-mono">256m</code>, <code className="font-mono">1g</code>, <code className="font-mono">2gb</code>
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="cpu_limit">CPU Limit</Label>
              <Input
                id="cpu_limit"
                placeholder="1.0"
                value={cpuLimit}
                onChange={(e) => setCpuLimit(e.target.value)}
              />
              <p className="text-xs text-muted-foreground">
                Number of CPU cores (e.g. <code className="font-mono">0.5</code>, <code className="font-mono">1</code>, <code className="font-mono">2</code>)
              </p>
            </div>
          </div>
          <div className="flex gap-2">
            <Button
              type="button"
              variant="destructive"
              onClick={handleApplyLimits}
              disabled={isApplyingLimits}
            >
              {isApplyingLimits ? "Applying..." : "Apply Now (Live)"}
            </Button>
            <p className="text-xs text-muted-foreground self-center">
              Saves limits and enforces them on the running container immediately via <code className="font-mono">docker update</code>
            </p>
          </div>
        </CardContent>
      </Card>

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
            {/* Restart Policy */}
            <div className="space-y-2">
              <Label htmlFor="restart_policy">Restart Policy</Label>
              <Select value={restartPolicy} onValueChange={setRestartPolicy}>
                <SelectTrigger id="restart_policy">
                  <SelectValue placeholder="Select restart policy" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="unless-stopped">unless-stopped (default)</SelectItem>
                  <SelectItem value="always">always</SelectItem>
                  <SelectItem value="on-failure">on-failure (max 5 retries)</SelectItem>
                  <SelectItem value="never">never</SelectItem>
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                Controls when Docker automatically restarts the container.{" "}
                <code className="font-mono">unless-stopped</code> restarts the container on crash
                but not after a manual stop.
              </p>
            </div>

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

            {/* GPU Access */}
            <div className="space-y-2">
              <Label htmlFor="gpus">GPU Access</Label>
              <Input
                id="gpus"
                placeholder="all"
                value={gpus}
                onChange={(e) => setGpus(e.target.value)}
              />
              <p className="text-xs text-muted-foreground">
                Grant access to GPUs. Use <code className="font-mono">all</code> for all GPUs,
                or <code className="font-mono">device=0,1</code> for specific devices. Requires
                the NVIDIA Container Toolkit on the host. Leave empty to disable.
              </p>
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

            {/* Cap Drop */}
            <div className="space-y-2">
              <Label htmlFor="cap_drop">Drop Capabilities</Label>
              <Textarea
                id="cap_drop"
                placeholder="MKNOD, NET_RAW"
                value={capDrop}
                onChange={(e) => setCapDrop(e.target.value)}
                rows={2}
              />
              <p className="text-xs text-muted-foreground">
                Comma-separated list of Linux capabilities to drop for extra hardening (e.g.{" "}
                <code className="font-mono">MKNOD, NET_RAW</code>).
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

            {/* Ulimits */}
            <div className="space-y-2">
              <Label htmlFor="ulimits">Ulimits</Label>
              <Textarea
                id="ulimits"
                placeholder="nofile=1024:1024"
                value={ulimits}
                onChange={(e) => setUlimits(e.target.value)}
                rows={3}
              />
              <p className="text-xs text-muted-foreground">
                One ulimit per line in the format{" "}
                <code className="font-mono">type=soft:hard</code> (e.g.{" "}
                <code className="font-mono">nofile=1024:1024</code>).
              </p>
            </div>

            {/* Security Options */}
            <div className="space-y-2">
              <Label htmlFor="security_opt">Security Options</Label>
              <Textarea
                id="security_opt"
                placeholder="seccomp=unconfined"
                value={securityOpt}
                onChange={(e) => setSecurityOpt(e.target.value)}
                rows={3}
              />
              <p className="text-xs text-muted-foreground">
                One security option per line (e.g.{" "}
                <code className="font-mono">seccomp=unconfined</code>,{" "}
                <code className="font-mono">apparmor=unconfined</code>).
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
