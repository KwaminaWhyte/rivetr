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
import { Plus, Trash2, Network, Server, Globe } from "lucide-react";
import type { PortMapping, App, UpdateAppRequest } from "@/types/api";

interface NetworkConfigCardProps {
  app: App;
  onSave: (updates: UpdateAppRequest) => Promise<void>;
  isSaving?: boolean;
}

// Helper functions to parse JSON strings from the app
function parsePortMappings(json: string | null): PortMapping[] {
  if (!json) return [];
  try {
    return JSON.parse(json);
  } catch {
    return [];
  }
}

function parseStringArray(json: string | null): string[] {
  if (!json) return [];
  try {
    return JSON.parse(json);
  } catch {
    return [];
  }
}

export function NetworkConfigCard({
  app,
  onSave,
  isSaving = false,
}: NetworkConfigCardProps) {
  // Parse current config from app
  const [portMappings, setPortMappings] = useState<PortMapping[]>(
    parsePortMappings(app.port_mappings)
  );
  const [networkAliases, setNetworkAliases] = useState<string[]>(
    parseStringArray(app.network_aliases)
  );
  const [extraHosts, setExtraHosts] = useState<string[]>(
    parseStringArray(app.extra_hosts)
  );

  // Form state for adding new items
  const [newPort, setNewPort] = useState({
    host_port: 0,
    container_port: 0,
    protocol: "tcp",
  });
  const [newAlias, setNewAlias] = useState("");
  const [newHost, setNewHost] = useState({ hostname: "", ip: "" });

  // Track if there are unsaved changes
  const [hasChanges, setHasChanges] = useState(false);

  // Update state when app changes
  useEffect(() => {
    setPortMappings(parsePortMappings(app.port_mappings));
    setNetworkAliases(parseStringArray(app.network_aliases));
    setExtraHosts(parseStringArray(app.extra_hosts));
    setHasChanges(false);
  }, [app.port_mappings, app.network_aliases, app.extra_hosts]);

  // Port mappings handlers
  const addPortMapping = () => {
    if (newPort.container_port < 1 || newPort.container_port > 65535) {
      toast.error("Container port must be between 1 and 65535");
      return;
    }
    if (newPort.host_port !== 0 && (newPort.host_port < 1024 || newPort.host_port > 65535)) {
      toast.error("Host port must be 0 (auto) or between 1024 and 65535");
      return;
    }

    const newMapping: PortMapping = {
      host_port: newPort.host_port,
      container_port: newPort.container_port,
      protocol: newPort.protocol,
    };

    setPortMappings([...portMappings, newMapping]);
    setNewPort({ host_port: 0, container_port: 0, protocol: "tcp" });
    setHasChanges(true);
  };

  const removePortMapping = (index: number) => {
    setPortMappings(portMappings.filter((_, i) => i !== index));
    setHasChanges(true);
  };

  // Network aliases handlers
  const addNetworkAlias = () => {
    if (!newAlias.trim()) {
      toast.error("Alias cannot be empty");
      return;
    }
    if (!/^[a-zA-Z0-9_-]+$/.test(newAlias)) {
      toast.error("Alias can only contain letters, numbers, dashes, and underscores");
      return;
    }
    if (networkAliases.includes(newAlias)) {
      toast.error("Alias already exists");
      return;
    }

    setNetworkAliases([...networkAliases, newAlias.trim()]);
    setNewAlias("");
    setHasChanges(true);
  };

  const removeNetworkAlias = (index: number) => {
    setNetworkAliases(networkAliases.filter((_, i) => i !== index));
    setHasChanges(true);
  };

  // Extra hosts handlers
  const addExtraHost = () => {
    if (!newHost.hostname.trim() || !newHost.ip.trim()) {
      toast.error("Both hostname and IP are required");
      return;
    }

    const hostEntry = `${newHost.hostname.trim()}:${newHost.ip.trim()}`;
    if (extraHosts.includes(hostEntry)) {
      toast.error("Host entry already exists");
      return;
    }

    setExtraHosts([...extraHosts, hostEntry]);
    setNewHost({ hostname: "", ip: "" });
    setHasChanges(true);
  };

  const removeExtraHost = (index: number) => {
    setExtraHosts(extraHosts.filter((_, i) => i !== index));
    setHasChanges(true);
  };

  // Save handler
  const handleSave = async () => {
    try {
      await onSave({
        port_mappings: portMappings,
        network_aliases: networkAliases,
        extra_hosts: extraHosts,
      });
      setHasChanges(false);
      toast.success("Network configuration saved");
    } catch (error) {
      toast.error(
        `Failed to save: ${error instanceof Error ? error.message : "Unknown error"}`
      );
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Network className="h-5 w-5" />
          Network Configuration
        </CardTitle>
        <CardDescription>
          Configure port mappings, network aliases, and extra hosts for container
          networking. Changes will take effect on the next deployment.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Port Mappings Section */}
        <div className="space-y-3">
          <div className="flex items-center gap-2">
            <Server className="h-4 w-4 text-muted-foreground" />
            <Label className="text-sm font-medium">Port Mappings</Label>
          </div>
          <p className="text-xs text-muted-foreground">
            Additional ports to expose from the container. Set host port to 0 for auto-assignment.
          </p>

          {portMappings.length > 0 && (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Host Port</TableHead>
                  <TableHead>Container Port</TableHead>
                  <TableHead>Protocol</TableHead>
                  <TableHead className="w-[60px]"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {portMappings.map((mapping, index) => (
                  <TableRow key={index}>
                    <TableCell className="font-mono">
                      {mapping.host_port === 0 ? "Auto" : mapping.host_port}
                    </TableCell>
                    <TableCell className="font-mono">
                      {mapping.container_port}
                    </TableCell>
                    <TableCell className="uppercase">{mapping.protocol}</TableCell>
                    <TableCell>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => removePortMapping(index)}
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

          <div className="flex gap-2 items-end">
            <div className="space-y-1">
              <Label className="text-xs">Host Port</Label>
              <Input
                type="number"
                placeholder="0 = auto"
                value={newPort.host_port || ""}
                onChange={(e) =>
                  setNewPort({ ...newPort, host_port: parseInt(e.target.value) || 0 })
                }
                className="w-24"
              />
            </div>
            <div className="space-y-1">
              <Label className="text-xs">Container Port</Label>
              <Input
                type="number"
                placeholder="8080"
                value={newPort.container_port || ""}
                onChange={(e) =>
                  setNewPort({
                    ...newPort,
                    container_port: parseInt(e.target.value) || 0,
                  })
                }
                className="w-24"
              />
            </div>
            <div className="space-y-1">
              <Label className="text-xs">Protocol</Label>
              <Select
                value={newPort.protocol}
                onValueChange={(value) => setNewPort({ ...newPort, protocol: value })}
              >
                <SelectTrigger className="w-20">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="tcp">TCP</SelectItem>
                  <SelectItem value="udp">UDP</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={addPortMapping}
              disabled={!newPort.container_port}
            >
              <Plus className="h-4 w-4 mr-1" />
              Add
            </Button>
          </div>
        </div>

        {/* Network Aliases Section */}
        <div className="space-y-3">
          <div className="flex items-center gap-2">
            <Globe className="h-4 w-4 text-muted-foreground" />
            <Label className="text-sm font-medium">Network Aliases</Label>
          </div>
          <p className="text-xs text-muted-foreground">
            DNS aliases for the container within Docker/Podman networks.
          </p>

          {networkAliases.length > 0 && (
            <div className="flex flex-wrap gap-2">
              {networkAliases.map((alias, index) => (
                <div
                  key={index}
                  className="flex items-center gap-1 bg-muted px-2 py-1 rounded-md"
                >
                  <span className="font-mono text-sm">{alias}</span>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => removeNetworkAlias(index)}
                    className="h-5 w-5 p-0 text-red-500 hover:text-red-600"
                  >
                    <Trash2 className="h-3 w-3" />
                  </Button>
                </div>
              ))}
            </div>
          )}

          <div className="flex gap-2">
            <Input
              placeholder="my-service"
              value={newAlias}
              onChange={(e) => setNewAlias(e.target.value)}
              className="max-w-[200px]"
              onKeyDown={(e) => e.key === "Enter" && addNetworkAlias()}
            />
            <Button
              variant="outline"
              size="sm"
              onClick={addNetworkAlias}
              disabled={!newAlias.trim()}
            >
              <Plus className="h-4 w-4 mr-1" />
              Add
            </Button>
          </div>
        </div>

        {/* Extra Hosts Section */}
        <div className="space-y-3">
          <div className="flex items-center gap-2">
            <Server className="h-4 w-4 text-muted-foreground" />
            <Label className="text-sm font-medium">Extra Hosts</Label>
          </div>
          <p className="text-xs text-muted-foreground">
            Custom host-to-IP mappings added to the container&apos;s /etc/hosts file.
            Use &quot;host-gateway&quot; as IP to access the host machine.
          </p>

          {extraHosts.length > 0 && (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Hostname</TableHead>
                  <TableHead>IP Address</TableHead>
                  <TableHead className="w-[60px]"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {extraHosts.map((host, index) => {
                  const [hostname, ip] = host.split(":");
                  return (
                    <TableRow key={index}>
                      <TableCell className="font-mono">{hostname}</TableCell>
                      <TableCell className="font-mono">{ip}</TableCell>
                      <TableCell>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => removeExtraHost(index)}
                          className="h-7 w-7 p-0 text-red-500 hover:text-red-600"
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          )}

          <div className="flex gap-2 items-end">
            <div className="space-y-1">
              <Label className="text-xs">Hostname</Label>
              <Input
                placeholder="myhost"
                value={newHost.hostname}
                onChange={(e) => setNewHost({ ...newHost, hostname: e.target.value })}
                className="w-40"
              />
            </div>
            <div className="space-y-1">
              <Label className="text-xs">IP Address</Label>
              <Input
                placeholder="192.168.1.1 or host-gateway"
                value={newHost.ip}
                onChange={(e) => setNewHost({ ...newHost, ip: e.target.value })}
                className="w-48"
              />
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={addExtraHost}
              disabled={!newHost.hostname.trim() || !newHost.ip.trim()}
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
              {isSaving ? "Saving..." : "Save Network Configuration"}
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
