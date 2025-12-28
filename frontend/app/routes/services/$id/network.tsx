import { useState, useMemo } from "react";
import { useOutletContext } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { toast } from "sonner";
import type { Service } from "@/types/api";
import { Copy, Check, Network, Server, ExternalLink, Globe, Container } from "lucide-react";

interface OutletContext {
  service: Service;
  token: string;
}

interface ParsedPort {
  hostPort: string;
  containerPort: string;
  protocol: string;
}

interface ParsedService {
  name: string;
  image: string;
  containerName: string;
  ports: ParsedPort[];
  networks: string[];
}

// Parse docker-compose YAML to extract network information
function parseComposeContent(content: string): ParsedService[] {
  try {
    const services: ParsedService[] = [];
    const lines = content.split("\n");

    let currentService: ParsedService | null = null;
    let inPorts = false;
    let inNetworks = false;
    let indent = 0;

    for (const line of lines) {
      const trimmed = line.trim();
      const lineIndent = line.search(/\S/);

      // Detect service name (under services: key)
      if (lineIndent === 2 && trimmed.endsWith(":") && !trimmed.startsWith("-")) {
        if (currentService) {
          services.push(currentService);
        }
        currentService = {
          name: trimmed.slice(0, -1),
          image: "",
          containerName: "",
          ports: [],
          networks: [],
        };
        inPorts = false;
        inNetworks = false;
      }

      // Parse image
      if (currentService && trimmed.startsWith("image:")) {
        currentService.image = trimmed.replace("image:", "").trim();
      }

      // Parse container_name
      if (currentService && trimmed.startsWith("container_name:")) {
        currentService.containerName = trimmed.replace("container_name:", "").trim();
      }

      // Detect ports section
      if (currentService && trimmed === "ports:") {
        inPorts = true;
        inNetworks = false;
        indent = lineIndent;
        continue;
      }

      // Detect networks section
      if (currentService && trimmed === "networks:") {
        inNetworks = true;
        inPorts = false;
        indent = lineIndent;
        continue;
      }

      // Parse port entries
      if (currentService && inPorts && trimmed.startsWith("-")) {
        const portStr = trimmed.slice(1).trim().replace(/"/g, "").replace(/'/g, "");
        const portMatch = portStr.match(/^(\d+):(\d+)(?:\/(\w+))?$/);
        if (portMatch) {
          currentService.ports.push({
            hostPort: portMatch[1],
            containerPort: portMatch[2],
            protocol: portMatch[3] || "tcp",
          });
        }
      }

      // Parse network entries
      if (currentService && inNetworks && trimmed.startsWith("-")) {
        const networkName = trimmed.slice(1).trim();
        currentService.networks.push(networkName);
      }

      // Reset section flags when indent decreases
      if (lineIndent <= indent && !trimmed.startsWith("-") && trimmed !== "") {
        inPorts = false;
        inNetworks = false;
      }
    }

    if (currentService) {
      services.push(currentService);
    }

    return services;
  } catch (e) {
    console.error("Failed to parse compose content:", e);
    return [];
  }
}

export default function ServiceNetworkTab() {
  const { service } = useOutletContext<OutletContext>();
  const [copiedField, setCopiedField] = useState<string | null>(null);

  const parsedServices = useMemo(
    () => parseComposeContent(service.compose_content),
    [service.compose_content]
  );

  // Collect all ports from all services
  const allPorts = useMemo(() => {
    const ports: Array<ParsedPort & { serviceName: string; containerName: string }> = [];
    for (const svc of parsedServices) {
      for (const port of svc.ports) {
        ports.push({
          ...port,
          serviceName: svc.name,
          containerName: svc.containerName || svc.name,
        });
      }
    }
    return ports;
  }, [parsedServices]);

  const copyToClipboard = (text: string, field: string) => {
    navigator.clipboard.writeText(text);
    setCopiedField(field);
    toast.success("Copied to clipboard");
    setTimeout(() => setCopiedField(null), 2000);
  };

  const CopyButton = ({ text, field }: { text: string; field: string }) => (
    <Button
      type="button"
      variant="ghost"
      size="icon"
      className="h-8 w-8"
      onClick={() => copyToClipboard(text, field)}
    >
      {copiedField === field ? (
        <Check className="h-4 w-4 text-green-500" />
      ) : (
        <Copy className="h-4 w-4" />
      )}
    </Button>
  );

  // Generate the compose project name (matches backend)
  const projectName = `rivetr-svc-${service.name.toLowerCase().replace(/[^a-z0-9]/g, "-")}`;

  return (
    <div className="space-y-6">
      {/* Exposed Ports Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Globe className="h-5 w-5" />
            Exposed Ports
          </CardTitle>
          <CardDescription>
            Ports exposed to the host machine
          </CardDescription>
        </CardHeader>
        <CardContent>
          {allPorts.length > 0 ? (
            <div className="space-y-4">
              {allPorts.map((port, idx) => (
                <div
                  key={idx}
                  className="flex items-center justify-between p-4 bg-muted rounded-lg"
                >
                  <div className="space-y-1">
                    <div className="flex items-center gap-2">
                      <span className="font-medium">Port {port.hostPort}</span>
                      <Badge variant="outline" className="text-xs">
                        {port.protocol.toUpperCase()}
                      </Badge>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      Host:{port.hostPort} → Container:{port.containerPort}
                    </p>
                  </div>
                  <div className="flex gap-2">
                    <CopyButton
                      text={`http://localhost:${port.hostPort}`}
                      field={`port-url-${idx}`}
                    />
                    {service.status === "running" && (
                      <Button variant="outline" size="sm" className="gap-1" asChild>
                        <a
                          href={`http://localhost:${port.hostPort}`}
                          target="_blank"
                          rel="noopener noreferrer"
                        >
                          <ExternalLink className="h-3 w-3" />
                          Open
                        </a>
                      </Button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-6 text-muted-foreground">
              <Network className="h-8 w-8 mx-auto mb-2 opacity-50" />
              <p>No ports are exposed to the host</p>
              <p className="text-sm mt-1">
                Add port mappings to your docker-compose.yml to expose services
              </p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Container Network Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Container className="h-5 w-5" />
            Container Network
          </CardTitle>
          <CardDescription>
            Docker network and container information
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>Compose Project Name</Label>
            <div className="flex gap-2">
              <Input value={projectName} readOnly className="font-mono" />
              <CopyButton text={projectName} field="project_name" />
            </div>
            <p className="text-xs text-muted-foreground">
              Used as prefix for container and network names
            </p>
          </div>

          <div className="space-y-2">
            <Label>Default Network</Label>
            <div className="flex gap-2">
              <Input value={`${projectName}_default`} readOnly className="font-mono" />
              <CopyButton text={`${projectName}_default`} field="network_name" />
            </div>
            <p className="text-xs text-muted-foreground">
              Containers in this service can communicate using service names as hostnames
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Container Details Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Server className="h-5 w-5" />
            Container Details
          </CardTitle>
          <CardDescription>
            Individual container names and network aliases
          </CardDescription>
        </CardHeader>
        <CardContent>
          {parsedServices.length > 0 ? (
            <div className="space-y-4">
              {parsedServices.map((svc, idx) => (
                <div key={idx} className="p-4 bg-muted rounded-lg space-y-3">
                  <div className="flex items-center justify-between">
                    <span className="font-medium">{svc.name}</span>
                    {svc.image && (
                      <code className="text-xs bg-background px-2 py-1 rounded">
                        {svc.image}
                      </code>
                    )}
                  </div>
                  <div className="grid gap-3 md:grid-cols-2">
                    <div className="space-y-1">
                      <Label className="text-xs">Container Name</Label>
                      <div className="flex gap-2">
                        <Input
                          value={svc.containerName || `${projectName}-${svc.name}-1`}
                          readOnly
                          className="font-mono text-sm h-8"
                        />
                        <CopyButton
                          text={svc.containerName || `${projectName}-${svc.name}-1`}
                          field={`container-${idx}`}
                        />
                      </div>
                    </div>
                    <div className="space-y-1">
                      <Label className="text-xs">Network Alias (Hostname)</Label>
                      <div className="flex gap-2">
                        <Input
                          value={svc.name}
                          readOnly
                          className="font-mono text-sm h-8"
                        />
                        <CopyButton text={svc.name} field={`alias-${idx}`} />
                      </div>
                    </div>
                  </div>
                  {svc.ports.length > 0 && (
                    <div className="text-xs text-muted-foreground">
                      Ports: {svc.ports.map(p => `${p.hostPort}:${p.containerPort}`).join(", ")}
                    </div>
                  )}
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-6 text-muted-foreground">
              <p>Unable to parse container details</p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Internal Communication Card */}
      <Card>
        <CardHeader>
          <CardTitle>Internal Communication</CardTitle>
          <CardDescription>
            How to connect to this service from other containers
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="rounded-md bg-muted p-4">
            <p className="text-sm mb-3">
              Other containers on the same Docker network can connect using:
            </p>
            <ul className="space-y-2 text-sm">
              {parsedServices.map((svc, idx) => (
                <li key={idx} className="flex items-center gap-2">
                  <Badge variant="outline" className="font-mono">
                    {svc.name}
                  </Badge>
                  <span className="text-muted-foreground">
                    {svc.ports.length > 0
                      ? `on port ${svc.ports[0].containerPort}`
                      : "(no exposed ports)"}
                  </span>
                </li>
              ))}
            </ul>
          </div>
          <p className="text-xs text-muted-foreground">
            Use the service name as the hostname when connecting from other containers
            within the same Docker Compose project or connected networks.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
