import { useMemo, useState } from "react";
import { useOutletContext } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { DetailField, StatusPill, type Tone } from "@/components/metrics";
import { toast } from "sonner";
import type { Service } from "@/types/api";
import {
  Clock,
  AlertCircle,
  Code,
  Calendar,
  ExternalLink,
  Network,
  HardDrive,
  Box,
  Copy,
  Check,
} from "lucide-react";

export function meta() {
  return [
    { title: "Service Overview - Rivetr" },
    { name: "description", content: "Service details and status overview" },
  ];
}

interface OutletContext {
  service: Service;
}

interface ParsedPort {
  hostPort: string;
  containerPort: string;
  protocol: string;
}

interface ParsedVolume {
  name: string;
  path: string;
  isNamed: boolean;
}

interface ParsedService {
  name: string;
  image: string;
  ports: ParsedPort[];
  volumes: ParsedVolume[];
}

function formatDate(dateString: string) {
  return new Date(dateString).toLocaleString();
}

// Parse docker-compose YAML to extract useful information
function parseComposeContent(content: string): ParsedService[] {
  try {
    // Simple YAML parsing for common patterns
    const services: ParsedService[] = [];
    const lines = content.split("\n");

    let currentService: ParsedService | null = null;
    let inPorts = false;
    let inVolumes = false;
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
          ports: [],
          volumes: [],
        };
        inPorts = false;
        inVolumes = false;
      }

      // Parse image
      if (currentService && trimmed.startsWith("image:")) {
        currentService.image = trimmed.replace("image:", "").trim();
      }

      // Detect ports section
      if (currentService && trimmed === "ports:") {
        inPorts = true;
        inVolumes = false;
        indent = lineIndent;
        continue;
      }

      // Detect volumes section
      if (currentService && trimmed === "volumes:") {
        inVolumes = true;
        inPorts = false;
        indent = lineIndent;
        continue;
      }

      // Parse port entries
      if (currentService && inPorts && trimmed.startsWith("-")) {
        const portStr = trimmed.slice(1).trim().replace(/"/g, "").replace(/'/g, "");
        // Parse port mapping like "8080:80" or "8080:80/tcp"
        const portMatch = portStr.match(/^(\d+):(\d+)(?:\/(\w+))?$/);
        if (portMatch) {
          currentService.ports.push({
            hostPort: portMatch[1],
            containerPort: portMatch[2],
            protocol: portMatch[3] || "tcp",
          });
        }
      }

      // Parse volume entries
      if (currentService && inVolumes && trimmed.startsWith("-")) {
        const volStr = trimmed.slice(1).trim().replace(/"/g, "").replace(/'/g, "");
        // Parse volume like "data:/app/data" or "/host/path:/container/path"
        const volParts = volStr.split(":");
        if (volParts.length >= 2) {
          const isNamed = !volParts[0].startsWith("/") && !volParts[0].startsWith(".");
          currentService.volumes.push({
            name: volParts[0],
            path: volParts[1],
            isNamed,
          });
        }
      }

      // Reset section flags when indent decreases
      if (lineIndent <= indent && !trimmed.startsWith("-") && trimmed !== "") {
        inPorts = false;
        inVolumes = false;
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

// Status indicator component
function StatusIndicator({ status }: { status: string }) {
  const statusConfig: Record<string, { tone: Tone; label: string }> = {
    running: { tone: "ok", label: "Running" },
    stopped: { tone: "neutral", label: "Stopped" },
    pending: { tone: "warn", label: "Pending" },
    starting: { tone: "warn", label: "Starting" },
    failed: { tone: "bad", label: "Failed" },
  };

  const config = statusConfig[status] || { tone: "neutral" as Tone, label: status };

  return <StatusPill tone={config.tone}>{config.label}</StatusPill>;
}

export default function ServiceGeneralTab() {
  const { service } = useOutletContext<OutletContext>();
  const [copiedField, setCopiedField] = useState<string | null>(null);

  const parsedServices = useMemo(
    () => parseComposeContent(service.compose_content),
    [service.compose_content]
  );

  // Collect all volumes
  const allVolumes = useMemo(() => {
    const volumes: Array<ParsedVolume & { serviceName: string }> = [];
    for (const svc of parsedServices) {
      for (const vol of svc.volumes) {
        volumes.push({ ...vol, serviceName: svc.name });
      }
    }
    return volumes;
  }, [parsedServices]);

  const copyToClipboard = (text: string, field: string) => {
    navigator.clipboard.writeText(text);
    setCopiedField(field);
    toast.success("Copied to clipboard");
    setTimeout(() => setCopiedField(null), 2000);
  };

  const CopyButton = ({ text, field }: { text: string; field: string }) => (
    <Button aria-label={copiedField === field ? "Copied" : "Copy to clipboard"}
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

  return (
    <div className="space-y-6">
      {/* Error Message if Failed */}
      {service.status === "failed" && service.error_message && (
        <Card className="border-destructive">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-destructive">
              <AlertCircle className="h-5 w-5" />
              Deployment Error
            </CardTitle>
          </CardHeader>
          <CardContent>
            <pre className="text-sm bg-muted p-4 rounded-lg overflow-x-auto whitespace-pre-wrap">
              {service.error_message}
            </pre>
          </CardContent>
        </Card>
      )}

      {/* Service URL - show domain link if configured, otherwise show port links */}
      {service.status === "running" && service.domain && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Network className="h-5 w-5" />
              Service URL
            </CardTitle>
            <CardDescription>
              Click to open the service in your browser
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex items-center justify-between p-3 bg-muted rounded-lg">
              <div className="flex flex-col min-w-0 flex-1">
                <span className="text-sm font-medium truncate">
                  {service.domain}
                </span>
                <span className="text-xs text-muted-foreground">
                  Routed via reverse proxy
                </span>
              </div>
              <div className="flex gap-1">
                <CopyButton
                  text={`https://${service.domain}`}
                  field="domain-url"
                />
                <Button
                  variant="outline"
                  size="sm"
                  className="gap-1"
                  asChild
                >
                  <a
                    href={`https://${service.domain}`}
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    <ExternalLink className="h-3 w-3" />
                    Open
                  </a>
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Service Info and Images */}
      <div className="grid gap-6 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Clock className="h-5 w-5" />
              Service Info
            </CardTitle>
            <CardDescription>General information about this service</CardDescription>
          </CardHeader>
          <CardContent>
            <dl className="grid gap-x-8 gap-y-5 sm:grid-cols-2">
              <DetailField label="Status">
                <StatusIndicator status={service.status} />
              </DetailField>
              <DetailField label="Created">{formatDate(service.created_at)}</DetailField>
              <DetailField label="Updated">{formatDate(service.updated_at)}</DetailField>
              <DetailField label="Service ID" mono className="sm:col-span-2">
                <span className="flex items-center gap-1">
                  <span className="truncate">{service.id}</span>
                  <CopyButton text={service.id} field="id" />
                </span>
              </DetailField>
            </dl>
          </CardContent>
        </Card>

        {/* Container Images */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Box className="h-5 w-5" />
              Container Images
            </CardTitle>
            <CardDescription>Docker images used by this service</CardDescription>
          </CardHeader>
          <CardContent>
            {parsedServices.length > 0 ? (
              <div className="space-y-3">
                {parsedServices.map((svc, idx) => (
                  <div
                    key={idx}
                    className="flex items-center justify-between p-3 bg-muted rounded-lg"
                  >
                    <div className="flex flex-col min-w-0 flex-1">
                      <span className="text-sm font-medium">{svc.name}</span>
                      <code className="text-xs text-muted-foreground truncate">
                        {svc.image || "build context"}
                      </code>
                    </div>
                    {svc.image && (
                      <CopyButton text={svc.image} field={`image-${idx}`} />
                    )}
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-sm text-muted-foreground text-center py-4">
                Unable to parse container images
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Volumes */}
      {allVolumes.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <HardDrive className="h-5 w-5" />
              Volumes
            </CardTitle>
            <CardDescription>Persistent storage mounted in containers</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="grid gap-3 sm:grid-cols-2">
              {allVolumes.map((vol, idx) => (
                <div
                  key={idx}
                  className="flex items-center justify-between p-3 bg-muted rounded-lg"
                >
                  <div className="flex flex-col min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium truncate">{vol.name}</span>
                      {vol.isNamed && (
                        <Badge variant="outline" className="text-xs">Named</Badge>
                      )}
                    </div>
                    <code className="text-xs text-muted-foreground truncate">
                      → {vol.path}
                    </code>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Docker Compose Configuration */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Code className="h-5 w-5" />
            Docker Compose Configuration
          </CardTitle>
          <CardDescription>The compose file used for this service</CardDescription>
        </CardHeader>
        <CardContent>
          <pre className="text-xs bg-muted p-4 rounded-lg overflow-x-auto max-h-96">
            <code>{service.compose_content}</code>
          </pre>
        </CardContent>
      </Card>
    </div>
  );
}
