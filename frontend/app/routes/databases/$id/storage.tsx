import { useState } from "react";
import { useOutletContext } from "react-router";
import type { ManagedDatabase } from "@/types/api";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { toast } from "sonner";
import { Copy, Check, HardDrive, FolderOpen, Database } from "lucide-react";

interface OutletContext {
  database: ManagedDatabase;
  token: string;
}

// Data paths for each database type
const DATA_PATHS: Record<string, { path: string; description: string }> = {
  postgres: {
    path: "/var/lib/postgresql/data",
    description: "PostgreSQL data directory containing databases and WAL files",
  },
  mysql: {
    path: "/var/lib/mysql",
    description: "MySQL data directory containing database files and logs",
  },
  mongodb: {
    path: "/data/db",
    description: "MongoDB data directory containing database files",
  },
  redis: {
    path: "/data",
    description: "Redis data directory for RDB snapshots and AOF files",
  },
};

export default function DatabaseStorageTab() {
  const { database, token } = useOutletContext<OutletContext>();
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

  const dataPathInfo = DATA_PATHS[database.db_type] || {
    path: "/data",
    description: "Database data directory",
  };

  return (
    <div className="space-y-6">
      {/* Volume Information Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <HardDrive className="h-5 w-5" />
            Persistent Volume
          </CardTitle>
          <CardDescription>
            Data persistence configuration for the database
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label>Volume Name</Label>
              <div className="flex gap-2">
                <Input
                  value={database.volume_name || "Not configured"}
                  readOnly
                  className="font-mono"
                />
                {database.volume_name && (
                  <CopyButton text={database.volume_name} field="volume_name" />
                )}
              </div>
              <p className="text-xs text-muted-foreground">
                The Docker volume name used for data persistence
              </p>
            </div>
            <div className="space-y-2">
              <Label>Host Path</Label>
              <div className="flex gap-2">
                <Input
                  value={database.volume_path || "Not configured"}
                  readOnly
                  className="font-mono text-sm"
                />
                {database.volume_path && (
                  <CopyButton text={database.volume_path} field="volume_path" />
                )}
              </div>
              <p className="text-xs text-muted-foreground">
                The host filesystem path where data is stored
              </p>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Container Paths Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <FolderOpen className="h-5 w-5" />
            Container Paths
          </CardTitle>
          <CardDescription>
            Data directories inside the container
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>Data Directory</Label>
            <div className="flex gap-2">
              <Input value={dataPathInfo.path} readOnly className="font-mono" />
              <CopyButton text={dataPathInfo.path} field="data_path" />
            </div>
            <p className="text-xs text-muted-foreground">{dataPathInfo.description}</p>
          </div>

          <div className="rounded-lg border p-4 bg-muted/50">
            <div className="flex items-start gap-3">
              <Database className="h-5 w-5 text-muted-foreground mt-0.5" />
              <div>
                <p className="text-sm font-medium">Volume Mount</p>
                <p className="text-sm text-muted-foreground mt-1">
                  {database.volume_path ? (
                    <>
                      <code className="bg-muted rounded px-1">{database.volume_path}</code>
                      {" → "}
                      <code className="bg-muted rounded px-1">{dataPathInfo.path}</code>
                    </>
                  ) : (
                    "No volume mounted - data will be lost when container is removed"
                  )}
                </p>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Data Persistence Info Card */}
      <Card>
        <CardHeader>
          <CardTitle>Data Persistence</CardTitle>
          <CardDescription>How your database data is preserved</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {database.volume_path ? (
            <>
              <div className="flex items-center gap-2">
                <Badge variant="default" className="bg-green-500">Persistent</Badge>
                <span className="text-sm text-muted-foreground">
                  Your data is stored on the host filesystem
                </span>
              </div>
              <div className="space-y-2 text-sm text-muted-foreground">
                <p>Your database data is persisted in the following ways:</p>
                <ul className="list-disc list-inside space-y-1 ml-2">
                  <li>Data survives container restarts</li>
                  <li>Data survives container recreation (stop → remove → create)</li>
                  <li>Data is stored at: <code className="bg-muted rounded px-1">{database.volume_path}</code></li>
                  <li>Backup this directory to preserve your data</li>
                </ul>
              </div>
            </>
          ) : (
            <>
              <div className="flex items-center gap-2">
                <Badge variant="destructive">Ephemeral</Badge>
                <span className="text-sm text-muted-foreground">
                  No persistent volume configured
                </span>
              </div>
              <div className="rounded-md bg-destructive/10 p-3">
                <p className="text-sm text-destructive">
                  <strong>Warning:</strong> Without a persistent volume, all data will be lost
                  when the container is removed. Consider configuring a volume mount.
                </p>
              </div>
            </>
          )}
        </CardContent>
      </Card>

      {/* Backup Information Card */}
      <Card>
        <CardHeader>
          <CardTitle>Backup & Restore</CardTitle>
          <CardDescription>Recommendations for data safety</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-4">
            <BackupInstructions database={database} />
          </div>
        </CardContent>
      </Card>

      {/* Storage Stats Card */}
      <Card>
        <CardHeader>
          <CardTitle>Storage Statistics</CardTitle>
          <CardDescription>Current storage usage</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="text-sm text-muted-foreground">
            <p>
              Storage statistics are not yet available. Use the terminal to check disk usage:
            </p>
            <pre className="mt-2 rounded-md bg-muted p-3 font-mono text-xs overflow-x-auto">
              {database.db_type === "postgres"
                ? `docker exec ${database.container_id || "CONTAINER_ID"} du -sh /var/lib/postgresql/data`
                : database.db_type === "mysql"
                ? `docker exec ${database.container_id || "CONTAINER_ID"} du -sh /var/lib/mysql`
                : database.db_type === "mongodb"
                ? `docker exec ${database.container_id || "CONTAINER_ID"} du -sh /data/db`
                : `docker exec ${database.container_id || "CONTAINER_ID"} du -sh /data`}
            </pre>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

// Backup instructions based on database type
function BackupInstructions({ database }: { database: ManagedDatabase }) {
  const [copiedField, setCopiedField] = useState<string | null>(null);

  const copyToClipboard = (text: string, field: string) => {
    navigator.clipboard.writeText(text);
    setCopiedField(field);
    toast.success("Copied to clipboard");
    setTimeout(() => setCopiedField(null), 2000);
  };

  const containerName = `rivetr-db-${database.name}`;
  const { username, database: dbName } = database.credentials || {};

  const backupCommands: Record<string, { label: string; command: string; description: string }[]> = {
    postgres: [
      {
        label: "Backup (pg_dump)",
        command: `docker exec ${containerName} pg_dump -U ${username} ${dbName || username} > backup.sql`,
        description: "Create a SQL dump of your database",
      },
      {
        label: "Restore",
        command: `docker exec -i ${containerName} psql -U ${username} ${dbName || username} < backup.sql`,
        description: "Restore from a SQL dump",
      },
    ],
    mysql: [
      {
        label: "Backup (mysqldump)",
        command: `docker exec ${containerName} mysqldump -u ${username} -p ${dbName || ""} > backup.sql`,
        description: "Create a SQL dump of your database",
      },
      {
        label: "Restore",
        command: `docker exec -i ${containerName} mysql -u ${username} -p ${dbName || ""} < backup.sql`,
        description: "Restore from a SQL dump",
      },
    ],
    mongodb: [
      {
        label: "Backup (mongodump)",
        command: `docker exec ${containerName} mongodump --out /data/backup && docker cp ${containerName}:/data/backup ./backup`,
        description: "Create a BSON dump of your databases",
      },
      {
        label: "Restore",
        command: `docker cp ./backup ${containerName}:/data/backup && docker exec ${containerName} mongorestore /data/backup`,
        description: "Restore from a BSON dump",
      },
    ],
    redis: [
      {
        label: "Backup (BGSAVE)",
        command: `docker exec ${containerName} redis-cli BGSAVE && docker cp ${containerName}:/data/dump.rdb ./dump.rdb`,
        description: "Create an RDB snapshot",
      },
      {
        label: "Restore",
        command: `docker cp ./dump.rdb ${containerName}:/data/dump.rdb && docker restart ${containerName}`,
        description: "Restore from an RDB snapshot",
      },
    ],
  };

  const commands = backupCommands[database.db_type] || [];

  return (
    <div className="space-y-4">
      {commands.map((cmd, idx) => (
        <div key={idx} className="space-y-2">
          <div className="flex items-center justify-between">
            <Label>{cmd.label}</Label>
            <Badge variant="outline">{cmd.description}</Badge>
          </div>
          <div className="flex gap-2">
            <pre className="flex-1 rounded-md bg-muted p-3 text-xs font-mono overflow-x-auto whitespace-pre-wrap break-all">
              {cmd.command}
            </pre>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="h-8 w-8 flex-shrink-0"
              onClick={() => copyToClipboard(cmd.command, `backup_${idx}`)}
            >
              {copiedField === `backup_${idx}` ? (
                <Check className="h-4 w-4 text-green-500" />
              ) : (
                <Copy className="h-4 w-4" />
              )}
            </Button>
          </div>
        </div>
      ))}

      {database.volume_path && (
        <div className="rounded-md bg-blue-50 p-3 dark:bg-blue-950">
          <p className="text-sm text-blue-700 dark:text-blue-300">
            <strong>Tip:</strong> You can also back up the entire data directory at{" "}
            <code className="bg-blue-100 dark:bg-blue-900 rounded px-1">{database.volume_path}</code>{" "}
            while the database is stopped for a complete backup.
          </p>
        </div>
      )}
    </div>
  );
}
