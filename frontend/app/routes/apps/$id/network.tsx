import { useState, useMemo } from "react";
import { useOutletContext } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { toast } from "sonner";
import type { App, Deployment, PortMapping, Domain } from "@/types/api";
import { Copy, Check, Globe, Server, ExternalLink, Network, Container, Lock } from "lucide-react";

interface OutletContext {
  app: App;
  deployments: Deployment[];
  token: string;
}

export default function AppNetworkTab() {
  const { app, deployments } = useOutletContext<OutletContext>();
  const [copiedField, setCopiedField] = useState<string | null>(null);

  const runningDeployment = deployments.find((d) => d.status === "running");

  // Parse port mappings from JSON string
  const portMappings: PortMapping[] = useMemo(() => {
    if (!app.port_mappings) return [];
    try {
      return JSON.parse(app.port_mappings);
    } catch {
      return [];
    }
  }, [app.port_mappings]);

  // Parse domains from JSON string
  const domains: Domain[] = useMemo(() => {
    if (!app.domains) return [];
    try {
      return JSON.parse(app.domains);
    } catch {
      return [];
    }
  }, [app.domains]);

  // Parse network aliases from JSON string
  const networkAliases: string[] = useMemo(() => {
    if (!app.network_aliases) return [];
    try {
      return JSON.parse(app.network_aliases);
    } catch {
      return [];
    }
  }, [app.network_aliases]);

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

  // Generate container name (matches backend naming)
  const containerName = runningDeployment
    ? `rivetr-${app.name}-${runningDeployment.id.slice(0, 8)}`
    : `rivetr-${app.name}-<deployment-id>`;

  return (
    <div className="space-y-6">
      {/* Domain Configuration */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Globe className="h-5 w-5" />
            Domain Configuration
          </CardTitle>
          <CardDescription>
            Public URLs for accessing your application
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Primary Domain */}
          <div className="space-y-2">
            <Label>Primary Domain</Label>
            <div className="flex gap-2">
              <Input
                value={app.domain || "Not configured"}
                readOnly
                className="font-mono"
              />
              {app.domain && (
                <>
                  <CopyButton text={app.domain} field="domain" />
                  <Button variant="outline" size="icon" asChild>
                    <a
                      href={`https://${app.domain}`}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      <ExternalLink className="h-4 w-4" />
                    </a>
                  </Button>
                </>
              )}
            </div>
            <p className="text-xs text-muted-foreground">
              The main domain used to access this application
            </p>
          </div>

          {/* Auto Subdomain */}
          {app.auto_subdomain && (
            <div className="space-y-2">
              <Label>Auto Subdomain</Label>
              <div className="flex gap-2">
                <Input
                  value={app.auto_subdomain}
                  readOnly
                  className="font-mono"
                />
                <CopyButton text={app.auto_subdomain} field="auto_subdomain" />
                <Button variant="outline" size="icon" asChild>
                  <a
                    href={`https://${app.auto_subdomain}`}
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    <ExternalLink className="h-4 w-4" />
                  </a>
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">
                Automatically assigned subdomain
              </p>
            </div>
          )}

          {/* Additional Domains */}
          {domains.length > 0 && (
            <div className="space-y-2">
              <Label>Additional Domains</Label>
              <div className="space-y-2">
                {domains.map((domain, idx) => (
                  <div key={idx} className="flex items-center gap-2 p-2 bg-muted rounded-lg">
                    <span className="font-mono text-sm flex-1">{domain.domain}</span>
                    {domain.primary && (
                      <Badge variant="outline" className="text-xs">Primary</Badge>
                    )}
                    <CopyButton text={domain.domain} field={`domain-${idx}`} />
                  </div>
                ))}
              </div>
            </div>
          )}

          {!app.domain && !app.auto_subdomain && domains.length === 0 && (
            <div className="text-center py-4 text-muted-foreground">
              <Globe className="h-8 w-8 mx-auto mb-2 opacity-50" />
              <p>No domain configured</p>
              <p className="text-sm">Configure a domain in the Settings tab</p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Port Configuration */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Server className="h-5 w-5" />
            Port Configuration
          </CardTitle>
          <CardDescription>
            Container port and host port mappings
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Application Port */}
          <div className="space-y-2">
            <Label>Application Port</Label>
            <div className="flex gap-2">
              <Input
                value={app.port}
                readOnly
                className="font-mono"
              />
              <CopyButton text={String(app.port)} field="app_port" />
            </div>
            <p className="text-xs text-muted-foreground">
              The port your application listens on inside the container
            </p>
          </div>

          {/* Additional Port Mappings */}
          {portMappings.length > 0 && (
            <div className="space-y-2">
              <Label>Additional Port Mappings</Label>
              <div className="space-y-2">
                {portMappings.map((mapping, idx) => (
                  <div
                    key={idx}
                    className="flex items-center justify-between p-3 bg-muted rounded-lg"
                  >
                    <div className="flex items-center gap-2">
                      <Badge variant="outline" className="font-mono">
                        {mapping.host_port}:{mapping.container_port}
                      </Badge>
                      <span className="text-sm text-muted-foreground">
                        /{mapping.protocol}
                      </span>
                    </div>
                    <CopyButton
                      text={`${mapping.host_port}:${mapping.container_port}`}
                      field={`port-${idx}`}
                    />
                  </div>
                ))}
              </div>
            </div>
          )}

          {portMappings.length === 0 && (
            <div className="rounded-md bg-muted p-3">
              <p className="text-sm text-muted-foreground">
                No additional port mappings configured. The application port ({app.port})
                is exposed through the reverse proxy.
              </p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Container Network */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Container className="h-5 w-5" />
            Container Network
          </CardTitle>
          <CardDescription>
            Docker container and network information
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label>Container Name</Label>
              <div className="flex gap-2">
                <Input
                  value={containerName}
                  readOnly
                  className="font-mono text-sm"
                />
                {runningDeployment && (
                  <CopyButton text={containerName} field="container_name" />
                )}
              </div>
            </div>
            <div className="space-y-2">
              <Label>Container ID</Label>
              <div className="flex gap-2">
                <Input
                  value={
                    runningDeployment?.container_id?.substring(0, 12) || "Not running"
                  }
                  readOnly
                  className="font-mono text-sm"
                />
                {runningDeployment?.container_id && (
                  <CopyButton
                    text={runningDeployment.container_id}
                    field="container_id"
                  />
                )}
              </div>
            </div>
          </div>

          {/* Network Aliases */}
          {networkAliases.length > 0 && (
            <div className="space-y-2">
              <Label>Network Aliases</Label>
              <div className="flex flex-wrap gap-2">
                {networkAliases.map((alias, idx) => (
                  <Badge key={idx} variant="outline" className="font-mono">
                    {alias}
                  </Badge>
                ))}
              </div>
              <p className="text-xs text-muted-foreground">
                Additional hostnames for reaching this container
              </p>
            </div>
          )}

          <div className="space-y-2">
            <Label>Network</Label>
            <Input value="rivetr-network" readOnly className="font-mono" />
            <p className="text-xs text-muted-foreground">
              All Rivetr apps share this Docker network for internal communication
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Internal Communication */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Lock className="h-5 w-5" />
            Internal Communication
          </CardTitle>
          <CardDescription>
            How other services can connect to this application
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>Internal URL</Label>
            <div className="flex gap-2">
              <Input
                value={
                  runningDeployment
                    ? `http://${containerName}:${app.port}`
                    : "App not running"
                }
                readOnly
                className="font-mono text-sm"
              />
              {runningDeployment && (
                <CopyButton
                  text={`http://${containerName}:${app.port}`}
                  field="internal_url"
                />
              )}
            </div>
            <p className="text-xs text-muted-foreground">
              Use this URL for service-to-service communication within Docker
            </p>
          </div>

          <div className="rounded-md bg-muted p-4">
            <p className="text-sm mb-2 font-medium">Connection Examples:</p>
            <div className="space-y-2 text-sm font-mono">
              <div className="p-2 bg-background rounded">
                # From another container
                <br />
                curl http://{runningDeployment ? containerName : "<container>"}:{app.port}
              </div>
              <div className="p-2 bg-background rounded">
                # Environment variable
                <br />
                {app.name.toUpperCase().replace(/-/g, "_")}_URL=http://
                {runningDeployment ? containerName : "<container>"}:{app.port}
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
