import { useState } from "react";
import { useOutletContext } from "react-router";
import type { ManagedDatabase, DATABASE_TYPES } from "@/types/api";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { ResourceMonitor } from "@/components/resource-monitor";
import { toast } from "sonner";
import { Eye, EyeOff, Copy, Check, ExternalLink } from "lucide-react";

interface OutletContext {
  database: ManagedDatabase;
}

// Database type descriptions
const DB_DESCRIPTIONS: Record<string, string> = {
  postgres: "The world's most advanced open source relational database",
  mysql: "The most popular open source relational database",
  mongodb: "A document-oriented NoSQL database",
  redis: "In-memory data structure store for caching and messaging",
};

export default function DatabaseGeneralTab() {
  const { database } = useOutletContext<OutletContext>();
  const [showPassword, setShowPassword] = useState(false);
  const [copiedField, setCopiedField] = useState<string | null>(null);

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

  return (
    <div className="space-y-6">
      {/* Database Info Card */}
      <Card>
        <CardHeader>
          <CardTitle>Database Information</CardTitle>
          <CardDescription>{DB_DESCRIPTIONS[database.db_type]}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label>Name</Label>
              <Input value={database.name} readOnly />
            </div>
            <div className="space-y-2">
              <Label>Type</Label>
              <Input value={database.db_type.toUpperCase()} readOnly />
            </div>
            <div className="space-y-2">
              <Label>Version</Label>
              <Input value={database.version} readOnly />
            </div>
            <div className="space-y-2">
              <Label>Status</Label>
              <div className="flex h-9 items-center">
                <StatusIndicator status={database.status} />
              </div>
            </div>
          </div>

          {database.error_message && (
            <div className="rounded-md bg-destructive/10 p-3 text-destructive">
              <Label className="text-destructive">Error</Label>
              <p className="mt-1 text-sm">{database.error_message}</p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Credentials Card */}
      <Card>
        <CardHeader>
          <CardTitle>Credentials</CardTitle>
          <CardDescription>Database connection credentials</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label>Username</Label>
              <div className="flex gap-2">
                <Input
                  value={database.credentials?.username || "-"}
                  readOnly
                  className="font-mono"
                />
                <CopyButton
                  text={database.credentials?.username || ""}
                  field="username"
                />
              </div>
            </div>
            <div className="space-y-2">
              <Label>Password</Label>
              <div className="flex gap-2">
                <Input
                  type={showPassword ? "text" : "password"}
                  value={database.credentials?.password || "-"}
                  readOnly
                  className="font-mono"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="h-9 w-9"
                  onClick={() => setShowPassword(!showPassword)}
                >
                  {showPassword ? (
                    <EyeOff className="h-4 w-4" />
                  ) : (
                    <Eye className="h-4 w-4" />
                  )}
                </Button>
                <CopyButton
                  text={database.credentials?.password || ""}
                  field="password"
                />
              </div>
            </div>
            {database.credentials?.database && (
              <div className="space-y-2">
                <Label>Database Name</Label>
                <div className="flex gap-2">
                  <Input
                    value={database.credentials.database}
                    readOnly
                    className="font-mono"
                  />
                  <CopyButton text={database.credentials.database} field="database" />
                </div>
              </div>
            )}
            {database.credentials?.root_password && database.db_type === "mysql" && (
              <div className="space-y-2">
                <Label>Root Password</Label>
                <div className="flex gap-2">
                  <Input
                    type={showPassword ? "text" : "password"}
                    value={database.credentials.root_password}
                    readOnly
                    className="font-mono"
                  />
                  <CopyButton
                    text={database.credentials.root_password}
                    field="root_password"
                  />
                </div>
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Connection Strings Card */}
      <Card>
        <CardHeader>
          <CardTitle>Connection Strings</CardTitle>
          <CardDescription>Use these to connect your applications</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {database.internal_connection_string && (
            <div className="space-y-2">
              <Label>Internal Connection (within Docker network)</Label>
              <div className="flex gap-2">
                <Input
                  value={database.internal_connection_string}
                  readOnly
                  className="font-mono text-sm"
                />
                <CopyButton
                  text={database.internal_connection_string}
                  field="internal_conn"
                />
              </div>
              <p className="text-xs text-muted-foreground">
                Use this connection string for applications running in the same Docker network
              </p>
            </div>
          )}
          {database.public_access && database.external_connection_string && (
            <div className="space-y-2">
              <Label className="flex items-center gap-2">
                External Connection
                <Badge variant="outline" className="text-xs">Public Access</Badge>
              </Label>
              <div className="flex gap-2">
                <Input
                  value={database.external_connection_string}
                  readOnly
                  className="font-mono text-sm"
                />
                <CopyButton
                  text={database.external_connection_string}
                  field="external_conn"
                />
              </div>
              <p className="text-xs text-muted-foreground">
                Use this connection string for external access from outside Docker
              </p>
            </div>
          )}
          {!database.public_access && (
            <div className="rounded-md bg-muted p-3">
              <p className="text-sm text-muted-foreground">
                External access is disabled. Enable public access in the Network tab to get an external connection string.
              </p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Resource Limits Card */}
      <Card>
        <CardHeader>
          <CardTitle>Resource Limits</CardTitle>
          <CardDescription>CPU and memory allocation</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label>Memory Limit</Label>
              <Input value={database.memory_limit || "512mb"} readOnly />
            </div>
            <div className="space-y-2">
              <Label>CPU Limit</Label>
              <Input value={database.cpu_limit || "0.5"} readOnly />
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Metadata Card */}
      <Card>
        <CardHeader>
          <CardTitle>Metadata</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label>Database ID</Label>
              <div className="flex gap-2">
                <Input value={database.id} readOnly className="font-mono text-xs" />
                <CopyButton text={database.id} field="id" />
              </div>
            </div>
            <div className="space-y-2">
              <Label>Container ID</Label>
              <div className="flex gap-2">
                <Input
                  value={database.container_id?.substring(0, 12) || "Not running"}
                  readOnly
                  className="font-mono text-xs"
                />
                {database.container_id && (
                  <CopyButton text={database.container_id} field="container_id" />
                )}
              </div>
            </div>
            <div className="space-y-2">
              <Label>Created</Label>
              <Input
                value={new Date(database.created_at).toLocaleString()}
                readOnly
              />
            </div>
            <div className="space-y-2">
              <Label>Last Updated</Label>
              <Input
                value={new Date(database.updated_at).toLocaleString()}
                readOnly
              />
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Resource Usage - only show when running */}
      {database.status === "running" && (
        <ResourceMonitor databaseId={database.id} />
      )}
    </div>
  );
}

// Status indicator component
function StatusIndicator({ status }: { status: string }) {
  const statusConfig: Record<string, { color: string; bg: string; label: string }> = {
    running: { color: "text-green-600", bg: "bg-green-100", label: "Running" },
    stopped: { color: "text-gray-600", bg: "bg-gray-100", label: "Stopped" },
    pending: { color: "text-blue-600", bg: "bg-blue-100", label: "Pending" },
    pulling: { color: "text-blue-600", bg: "bg-blue-100", label: "Pulling Image" },
    starting: { color: "text-blue-600", bg: "bg-blue-100", label: "Starting" },
    failed: { color: "text-red-600", bg: "bg-red-100", label: "Failed" },
  };

  const config = statusConfig[status] || {
    color: "text-gray-600",
    bg: "bg-gray-100",
    label: status,
  };

  return (
    <Badge variant="outline" className={`${config.bg} ${config.color} border-0`}>
      {["pending", "pulling", "starting"].includes(status) && (
        <span className="mr-1.5 relative flex h-2 w-2">
          <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-blue-400 opacity-75"></span>
          <span className="relative inline-flex h-2 w-2 rounded-full bg-blue-500"></span>
        </span>
      )}
      {config.label}
    </Badge>
  );
}
