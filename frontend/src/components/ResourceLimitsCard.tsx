import { useState, useEffect } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { api } from "@/lib/api";
import type { App, UpdateAppRequest } from "@/types/api";

// CPU options from 0.25 to 4 cores, step 0.25
const CPU_OPTIONS = [
  { value: "0.25", label: "0.25 cores" },
  { value: "0.5", label: "0.5 cores" },
  { value: "0.75", label: "0.75 cores" },
  { value: "1", label: "1 core" },
  { value: "1.25", label: "1.25 cores" },
  { value: "1.5", label: "1.5 cores" },
  { value: "1.75", label: "1.75 cores" },
  { value: "2", label: "2 cores" },
  { value: "2.5", label: "2.5 cores" },
  { value: "3", label: "3 cores" },
  { value: "3.5", label: "3.5 cores" },
  { value: "4", label: "4 cores" },
];

// Memory options from 128MB to 4GB
const MEMORY_OPTIONS = [
  { value: "128m", label: "128 MB" },
  { value: "256m", label: "256 MB" },
  { value: "512m", label: "512 MB" },
  { value: "768m", label: "768 MB" },
  { value: "1g", label: "1 GB" },
  { value: "1536m", label: "1.5 GB" },
  { value: "2g", label: "2 GB" },
  { value: "3g", label: "3 GB" },
  { value: "4g", label: "4 GB" },
];

interface ResourceLimitsCardProps {
  app: App;
}

export function ResourceLimitsCard({ app }: ResourceLimitsCardProps) {
  const queryClient = useQueryClient();
  const [cpuLimit, setCpuLimit] = useState<string>(app.cpu_limit || "1");
  const [memoryLimit, setMemoryLimit] = useState<string>(app.memory_limit || "512m");
  const [hasChanges, setHasChanges] = useState(false);

  // Reset values when app changes
  useEffect(() => {
    setCpuLimit(app.cpu_limit || "1");
    setMemoryLimit(app.memory_limit || "512m");
    setHasChanges(false);
  }, [app.cpu_limit, app.memory_limit]);

  // Track changes
  useEffect(() => {
    const cpuChanged = cpuLimit !== (app.cpu_limit || "1");
    const memoryChanged = memoryLimit !== (app.memory_limit || "512m");
    setHasChanges(cpuChanged || memoryChanged);
  }, [cpuLimit, memoryLimit, app.cpu_limit, app.memory_limit]);

  const updateMutation = useMutation({
    mutationFn: (data: UpdateAppRequest) => api.updateApp(app.id, data),
    onSuccess: () => {
      toast.success("Resource limits updated");
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
      setHasChanges(false);
    },
    onError: (error: Error) => {
      toast.error(`Failed to update resource limits: ${error.message}`);
    },
  });

  const handleSave = () => {
    updateMutation.mutate({
      cpu_limit: cpuLimit,
      memory_limit: memoryLimit,
    });
  };

  const handleReset = () => {
    setCpuLimit(app.cpu_limit || "1");
    setMemoryLimit(app.memory_limit || "512m");
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Resource Limits</CardTitle>
        <CardDescription>
          Configure CPU and memory limits for this application's container
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        <div className="grid gap-6 md:grid-cols-2">
          <div className="space-y-2">
            <Label htmlFor="cpu-limit">CPU Limit</Label>
            <Select value={cpuLimit} onValueChange={setCpuLimit}>
              <SelectTrigger className="w-full">
                <SelectValue placeholder="Select CPU limit" />
              </SelectTrigger>
              <SelectContent>
                {CPU_OPTIONS.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">
              Maximum CPU cores this container can use
            </p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="memory-limit">Memory Limit</Label>
            <Select value={memoryLimit} onValueChange={setMemoryLimit}>
              <SelectTrigger className="w-full">
                <SelectValue placeholder="Select memory limit" />
              </SelectTrigger>
              <SelectContent>
                {MEMORY_OPTIONS.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">
              Maximum memory this container can use
            </p>
          </div>
        </div>

        <div className="flex gap-2">
          <Button
            onClick={handleSave}
            disabled={!hasChanges || updateMutation.isPending}
          >
            {updateMutation.isPending ? "Saving..." : "Save Changes"}
          </Button>
          {hasChanges && (
            <Button variant="outline" onClick={handleReset}>
              Reset
            </Button>
          )}
        </div>

        <p className="text-xs text-muted-foreground">
          Changes will take effect on the next deployment.
        </p>
      </CardContent>
    </Card>
  );
}

// Export constants for reuse in other components
export { CPU_OPTIONS, MEMORY_OPTIONS };
