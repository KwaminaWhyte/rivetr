import { useState } from "react";
import { useOutletContext } from "react-router";
import type { ManagedDatabase } from "@/types/api";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { toast } from "sonner";
import { Copy, Check, Globe, Lock, Server, Container, Network } from "lucide-react";

interface OutletContext {
  database: ManagedDatabase;
  token: string;
}

// Port defaults for each database type
const DEFAULT_PORTS: Record<string, number> = {
  postgres: 5432,
  mysql: 3306,
  mongodb: 27017,
  redis: 6379,
};

export default function DatabaseNetworkTab() {
  const { database } = useOutletContext<OutletContext>();
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

  const defaultPort = DEFAULT_PORTS[database.db_type] || 5432;

  return (
    <div className="space-y-6">
      {/* Port Configuration Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Server className="h-5 w-5" />
            Port Configuration
          </CardTitle>
          <CardDescription>Database port mappings and networking</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label>Internal Port (Container)</Label>
              <div className="flex gap-2">
                <Input
                  value={database.internal_port || defaultPort}
                  readOnly
                  className="font-mono"
                />
                <CopyButton
                  text={String(database.internal_port || defaultPort)}
                  field="internal_port"
                />
              </div>
              <p className="text-xs text-muted-foreground">
                The port the database listens on inside the container
              </p>
            </div>
            <div className="space-y-2">
              <Label>External Port (Host)</Label>
              <div className="flex gap-2">
                <Input
                  value={
                    database.public_access && database.external_port
                      ? database.external_port
                      : "Not exposed"
                  }
                  readOnly
                  className="font-mono"
                />
                {database.public_access && database.external_port && (
                  <CopyButton
                    text={String(database.external_port)}
                    field="external_port"
                  />
                )}
              </div>
              <p className="text-xs text-muted-foreground">
                {database.public_access
                  ? "The host port mapped for external access"
                  : "Enable public access to expose an external port"}
              </p>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Public Access Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            {database.public_access ? (
              <Globe className="h-5 w-5 text-green-500" />
            ) : (
              <Lock className="h-5 w-5 text-muted-foreground" />
            )}
            Public Access
          </CardTitle>
          <CardDescription>
            Control whether this database is accessible from outside the Docker network
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between rounded-lg border p-4">
            <div className="space-y-0.5">
              <Label className="text-base">Enable Public Access</Label>
              <p className="text-sm text-muted-foreground">
                {database.public_access
                  ? "Database is accessible from the host machine and external networks"
                  : "Database is only accessible within the Docker network"}
              </p>
            </div>
            <Badge
              variant={database.public_access ? "default" : "secondary"}
              className={database.public_access ? "bg-green-500" : ""}
            >
              {database.public_access ? "Enabled" : "Disabled"}
            </Badge>
          </div>

          <div className="rounded-md bg-muted p-3">
            <p className="text-sm text-muted-foreground">
              <strong>Note:</strong> Public access settings can be modified in the Settings tab.
              Changing this requires restarting the database container.
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Connection URLs Card */}
      <Card>
        <CardHeader>
          <CardTitle>Connection URLs</CardTitle>
          <CardDescription>Ready-to-use connection strings for your applications</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Internal URL */}
          <div className="space-y-2">
            <Label className="flex items-center gap-2">
              <Lock className="h-4 w-4" />
              Internal URL (Docker Network)
            </Label>
            {database.internal_connection_string ? (
              <div className="flex gap-2">
                <Input
                  value={database.internal_connection_string}
                  readOnly
                  className="font-mono text-sm"
                />
                <CopyButton
                  text={database.internal_connection_string}
                  field="internal_url"
                />
              </div>
            ) : (
              <Input value="Not available - database not running" readOnly />
            )}
            <p className="text-xs text-muted-foreground">
              Use this URL for services running in the same Docker network
            </p>
          </div>

          {/* External URL */}
          <div className="space-y-2">
            <Label className="flex items-center gap-2">
              <Globe className="h-4 w-4" />
              External URL (Host Access)
              {database.public_access && (
                <Badge variant="outline" className="text-xs bg-green-50 text-green-700 border-green-200">
                  Public
                </Badge>
              )}
            </Label>
            {database.public_access && database.external_connection_string ? (
              <div className="flex gap-2">
                <Input
                  value={database.external_connection_string}
                  readOnly
                  className="font-mono text-sm"
                />
                <CopyButton
                  text={database.external_connection_string}
                  field="external_url"
                />
              </div>
            ) : (
              <Input
                value={
                  database.public_access
                    ? "Not available - database not running"
                    : "Public access disabled"
                }
                readOnly
              />
            )}
            <p className="text-xs text-muted-foreground">
              {database.public_access
                ? "Use this URL for external access from the host machine or external networks"
                : "Enable public access to get an external connection URL"}
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Container Network Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Container className="h-5 w-5" />
            Container Network
          </CardTitle>
          <CardDescription>Docker container and network information</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label>Container Name</Label>
              <div className="flex gap-2">
                <Input
                  value={`rivetr-db-${database.name}`}
                  readOnly
                  className="font-mono"
                />
                <CopyButton text={`rivetr-db-${database.name}`} field="container_name" />
              </div>
            </div>
            <div className="space-y-2">
              <Label>Container ID</Label>
              <div className="flex gap-2">
                <Input
                  value={database.container_id?.substring(0, 12) || "Not running"}
                  readOnly
                  className="font-mono"
                />
                {database.container_id && (
                  <CopyButton text={database.container_id} field="container_id" />
                )}
              </div>
            </div>
          </div>

          <div className="space-y-2">
            <Label>Internal Hostname</Label>
            <div className="flex gap-2">
              <Input
                value={`rivetr-db-${database.name}`}
                readOnly
                className="font-mono"
              />
              <CopyButton text={`rivetr-db-${database.name}`} field="internal_hostname" />
            </div>
            <p className="text-xs text-muted-foreground">
              Other containers can reach this database using this hostname for service-to-service communication
            </p>
          </div>

          <div className="space-y-2">
            <Label className="flex items-center gap-2">
              <Network className="h-4 w-4" />
              Docker Network
            </Label>
            <div className="flex gap-2">
              <Input
                value="rivetr-network"
                readOnly
                className="font-mono"
              />
              <CopyButton text="rivetr-network" field="network_name" />
            </div>
            <p className="text-xs text-muted-foreground">
              All Rivetr services share this network for internal communication
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Connection Examples Card */}
      <Card>
        <CardHeader>
          <CardTitle>Connection Examples</CardTitle>
          <CardDescription>
            Quick reference for connecting to your {database.db_type.toUpperCase()} database
          </CardDescription>
        </CardHeader>
        <CardContent>
          <ConnectionExamples database={database} />
        </CardContent>
      </Card>

      {/* Environment Variables Card */}
      <Card>
        <CardHeader>
          <CardTitle>Environment Variable Examples</CardTitle>
          <CardDescription>
            How to configure other containers to connect to this database
          </CardDescription>
        </CardHeader>
        <CardContent>
          <EnvironmentVariableExamples database={database} />
        </CardContent>
      </Card>
    </div>
  );
}

// Connection examples based on database type
function ConnectionExamples({ database }: { database: ManagedDatabase }) {
  const [copiedField, setCopiedField] = useState<string | null>(null);

  const copyToClipboard = (text: string, field: string) => {
    navigator.clipboard.writeText(text);
    setCopiedField(field);
    toast.success("Copied to clipboard");
    setTimeout(() => setCopiedField(null), 2000);
  };

  const containerName = `rivetr-db-${database.name}`;
  const { username, password, database: dbName } = database.credentials || {};
  const port = database.internal_port || DEFAULT_PORTS[database.db_type];
  const externalPort = database.external_port;

  const examples: Record<string, { label: string; command: string }[]> = {
    postgres: [
      {
        label: "psql (internal)",
        command: `psql -h ${containerName} -p ${port} -U ${username} -d ${dbName || username}`,
      },
      {
        label: "psql (external)",
        command: database.public_access && externalPort
          ? `psql -h localhost -p ${externalPort} -U ${username} -d ${dbName || username}`
          : "Enable public access for external connection",
      },
      {
        label: "Node.js",
        command: `const { Pool } = require('pg');\nconst pool = new Pool({ connectionString: '${database.internal_connection_string || "postgresql://..."}' });`,
      },
    ],
    mysql: [
      {
        label: "mysql (internal)",
        command: `mysql -h ${containerName} -P ${port} -u ${username} -p ${dbName || ""}`,
      },
      {
        label: "mysql (external)",
        command: database.public_access && externalPort
          ? `mysql -h localhost -P ${externalPort} -u ${username} -p ${dbName || ""}`
          : "Enable public access for external connection",
      },
      {
        label: "Node.js",
        command: `const mysql = require('mysql2/promise');\nconst conn = await mysql.createConnection('${database.internal_connection_string || "mysql://..."}');`,
      },
    ],
    mongodb: [
      {
        label: "mongosh (internal)",
        command: `mongosh "${database.internal_connection_string || `mongodb://${username}:***@${containerName}:${port}/`}"`,
      },
      {
        label: "mongosh (external)",
        command: database.public_access && externalPort
          ? `mongosh "mongodb://${username}:***@localhost:${externalPort}/"`
          : "Enable public access for external connection",
      },
      {
        label: "Node.js",
        command: `const { MongoClient } = require('mongodb');\nconst client = new MongoClient('${database.internal_connection_string || "mongodb://..."}');`,
      },
    ],
    redis: [
      {
        label: "redis-cli (internal)",
        command: `redis-cli -h ${containerName} -p ${port}`,
      },
      {
        label: "redis-cli (external)",
        command: database.public_access && externalPort
          ? `redis-cli -h localhost -p ${externalPort}`
          : "Enable public access for external connection",
      },
      {
        label: "Node.js",
        command: `const Redis = require('ioredis');\nconst redis = new Redis('${database.internal_connection_string || "redis://..."}');`,
      },
    ],
  };

  const dbExamples = examples[database.db_type] || [];

  return (
    <div className="space-y-4">
      {dbExamples.map((example, idx) => (
        <div key={idx} className="space-y-2">
          <Label>{example.label}</Label>
          <div className="flex gap-2">
            <pre className="flex-1 rounded-md bg-muted p-3 text-sm font-mono overflow-x-auto whitespace-pre-wrap break-all">
              {example.command}
            </pre>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="h-8 w-8 flex-shrink-0"
              onClick={() => copyToClipboard(example.command, `example_${idx}`)}
            >
              {copiedField === `example_${idx}` ? (
                <Check className="h-4 w-4 text-green-500" />
              ) : (
                <Copy className="h-4 w-4" />
              )}
            </Button>
          </div>
        </div>
      ))}
    </div>
  );
}

// Environment variable examples for connecting from other containers
function EnvironmentVariableExamples({ database }: { database: ManagedDatabase }) {
  const [copiedField, setCopiedField] = useState<string | null>(null);

  const copyToClipboard = (text: string, field: string) => {
    navigator.clipboard.writeText(text);
    setCopiedField(field);
    toast.success("Copied to clipboard");
    setTimeout(() => setCopiedField(null), 2000);
  };

  const containerName = `rivetr-db-${database.name}`;
  const { username, password, database: dbName } = database.credentials || {};
  const port = database.internal_port || DEFAULT_PORTS[database.db_type];

  // Environment variable name (uppercase, replace dashes with underscores)
  const envVarPrefix = database.name.toUpperCase().replace(/-/g, "_");

  // Build individual environment variable lines
  const envVars = [
    { name: `${envVarPrefix}_HOST`, value: containerName },
    { name: `${envVarPrefix}_PORT`, value: String(port) },
    { name: `${envVarPrefix}_USER`, value: username || "user" },
    { name: `${envVarPrefix}_PASSWORD`, value: password || "********" },
  ];

  // Add database name for non-Redis databases
  if (database.db_type !== "redis" && dbName) {
    envVars.push({ name: `${envVarPrefix}_DATABASE`, value: dbName });
  }

  // Connection string env var
  const connectionStringEnvVar = {
    name: `${envVarPrefix}_URL`,
    value: database.internal_connection_string || "Connection string not available",
  };

  return (
    <div className="space-y-4">
      {/* Connection String */}
      <div className="rounded-md bg-muted p-4">
        <p className="text-sm mb-3 font-medium">For Rivetr Apps (Internal Network):</p>
        <div className="space-y-2 text-sm font-mono">
          <div className="p-2 bg-background rounded flex items-center justify-between gap-2">
            <span className="break-all">{connectionStringEnvVar.name}={connectionStringEnvVar.value}</span>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="h-8 w-8 flex-shrink-0"
              onClick={() => copyToClipboard(`${connectionStringEnvVar.name}=${connectionStringEnvVar.value}`, "env_url")}
            >
              {copiedField === "env_url" ? (
                <Check className="h-4 w-4 text-green-500" />
              ) : (
                <Copy className="h-4 w-4" />
              )}
            </Button>
          </div>
        </div>
      </div>

      {/* Individual Variables */}
      <div className="rounded-md bg-muted p-4">
        <p className="text-sm mb-3 font-medium">Individual Variables:</p>
        <div className="space-y-2 text-sm font-mono">
          {envVars.map((envVar, idx) => (
            <div key={idx} className="p-2 bg-background rounded flex items-center justify-between gap-2">
              <span className="break-all">{envVar.name}={envVar.value}</span>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="h-8 w-8 flex-shrink-0"
                onClick={() => copyToClipboard(`${envVar.name}=${envVar.value}`, `env_${idx}`)}
              >
                {copiedField === `env_${idx}` ? (
                  <Check className="h-4 w-4 text-green-500" />
                ) : (
                  <Copy className="h-4 w-4" />
                )}
              </Button>
            </div>
          ))}
        </div>
      </div>

      {/* External Access Note */}
      {database.public_access && database.external_connection_string && (
        <div className="rounded-md bg-muted p-4">
          <p className="text-sm mb-3 font-medium">For External Services:</p>
          <div className="space-y-2 text-sm font-mono">
            <div className="p-2 bg-background rounded flex items-center justify-between gap-2">
              <span className="break-all">{envVarPrefix}_URL={database.external_connection_string}</span>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="h-8 w-8 flex-shrink-0"
                onClick={() => copyToClipboard(`${envVarPrefix}_URL=${database.external_connection_string}`, "env_external_url")}
              >
                {copiedField === "env_external_url" ? (
                  <Check className="h-4 w-4 text-green-500" />
                ) : (
                  <Copy className="h-4 w-4" />
                )}
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Tip */}
      <div className="rounded-md border p-3">
        <p className="text-sm text-muted-foreground">
          <strong>Tip:</strong> When connecting from another Rivetr app, use the internal hostname (<code className="bg-muted px-1 rounded">{containerName}</code>) instead of <code className="bg-muted px-1 rounded">localhost</code> for better performance and security.
        </p>
      </div>
    </div>
  );
}
