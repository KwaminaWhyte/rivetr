import { useState, useEffect } from "react";
import { useOutletContext } from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { DomainManagementCard } from "@/components/domain-management-card";
import { NetworkConfigCard } from "@/components/network-config-card";
import { ContainerLabelsCard } from "@/components/container-labels-card";
import { api } from "@/lib/api";
import { destinationsApi } from "@/lib/api/destinations";
import type { App, UpdateAppRequest } from "@/types/api";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Loader2, Route } from "lucide-react";

export default function AppSettingsNetwork() {
  const { app } = useOutletContext<{ app: App }>();
  const queryClient = useQueryClient();
  const [isSavingNetwork, setIsSavingNetwork] = useState(false);
  const [isSavingDomains, setIsSavingDomains] = useState(false);
  const [isSavingLabels, setIsSavingLabels] = useState(false);
  const [isSavingPrefix, setIsSavingPrefix] = useState(false);
  const [stripPrefix, setStripPrefix] = useState(app.strip_prefix ?? "");
  const [isSavingDestination, setIsSavingDestination] = useState(false);
  const [destinationId, setDestinationId] = useState(app.destination_id ?? "");

  const { data: destinations } = useQuery({
    queryKey: ["destinations"],
    queryFn: () => destinationsApi.list(),
  });

  useEffect(() => {
    setStripPrefix(app.strip_prefix ?? "");
    setDestinationId(app.destination_id ?? "");
  }, [app.strip_prefix, app.destination_id]);

  const handleSaveNetworkConfig = async (updates: UpdateAppRequest) => {
    setIsSavingNetwork(true);
    try {
      await api.updateApp(app.id, updates);
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } finally {
      setIsSavingNetwork(false);
    }
  };

  const handleSaveDomainConfig = async (updates: UpdateAppRequest) => {
    setIsSavingDomains(true);
    try {
      await api.updateApp(app.id, updates);
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } finally {
      setIsSavingDomains(false);
    }
  };

  const handleSaveContainerLabels = async (updates: UpdateAppRequest) => {
    setIsSavingLabels(true);
    try {
      await api.updateApp(app.id, updates);
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } finally {
      setIsSavingLabels(false);
    }
  };

  const handleSaveStripPrefix = async () => {
    setIsSavingPrefix(true);
    try {
      await api.updateApp(app.id, { strip_prefix: stripPrefix });
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
      toast.success("Strip prefix saved");
    } catch (error) {
      toast.error(
        `Failed to save: ${error instanceof Error ? error.message : "Unknown error"}`
      );
    } finally {
      setIsSavingPrefix(false);
    }
  };

  const handleSaveDestination = async () => {
    setIsSavingDestination(true);
    try {
      await api.updateApp(app.id, { destination_id: destinationId || "" });
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
      toast.success("Destination updated");
    } catch (error) {
      toast.error(`Failed to save: ${error instanceof Error ? error.message : "Unknown error"}`);
    } finally {
      setIsSavingDestination(false);
    }
  };

  return (
    <div className="space-y-6">
      <DomainManagementCard app={app} onSave={handleSaveDomainConfig} isSaving={isSavingDomains} />
      <NetworkConfigCard app={app} onSave={handleSaveNetworkConfig} isSaving={isSavingNetwork} />

      {/* Strip URL Prefix */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Route className="h-5 w-5" />
            Strip URL Prefix
          </CardTitle>
          <CardDescription>
            Remove this path prefix from incoming requests before forwarding to the container (e.g. /api). Leave empty to disable.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="strip-prefix">Strip Prefix</Label>
            <Input
              id="strip-prefix"
              placeholder="/api"
              value={stripPrefix}
              onChange={(e) => setStripPrefix(e.target.value)}
              className="max-w-sm"
            />
            <p className="text-xs text-muted-foreground">
              If set, requests to <code className="font-mono">/api/users</code> will be forwarded as <code className="font-mono">/users</code>. Changes take effect immediately without redeployment.
            </p>
          </div>
          <Button
            onClick={handleSaveStripPrefix}
            disabled={isSavingPrefix}
            size="sm"
            className="gap-2"
          >
            {isSavingPrefix ? <Loader2 className="h-4 w-4 animate-spin" /> : null}
            Save
          </Button>
        </CardContent>
      </Card>

      <ContainerLabelsCard app={app} onSave={handleSaveContainerLabels} isSaving={isSavingLabels} />

      {/* Docker Destination */}
      <Card>
        <CardHeader>
          <CardTitle>Docker Destination</CardTitle>
          <CardDescription>
            Assign this app to a named Docker network. Leave as default to use the shared <code className="text-xs">rivetr</code> bridge network.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <Select value={destinationId} onValueChange={setDestinationId}>
            <SelectTrigger className="max-w-sm">
              <SelectValue placeholder="Default (rivetr network)" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="">Default (rivetr network)</SelectItem>
              {destinations?.map((d) => (
                <SelectItem key={d.id} value={d.id}>
                  {d.name} ({d.network_name})
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button
            onClick={handleSaveDestination}
            disabled={isSavingDestination}
            size="sm"
            className="gap-2"
          >
            {isSavingDestination ? <Loader2 className="h-4 w-4 animate-spin" /> : null}
            Save Destination
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
