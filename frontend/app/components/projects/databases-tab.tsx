import { useState } from "react";
import { Link } from "react-router";
import { useQueryClient, useMutation } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  AlertCircle,
  ChevronDown,
  Copy,
  Database,
  Eye,
  EyeOff,
  Play,
  Plus,
  Square,
  Trash2,
} from "lucide-react";
import { api } from "@/lib/api";
import { useTeamContext } from "@/lib/team-context";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Badge } from "@/components/ui/badge";
import type { ProjectWithApps, ManagedDatabase, DatabaseType } from "@/types/api";
import { DATABASE_TYPES } from "@/types/api";

interface DatabasesTabProps {
  project: ProjectWithApps;
  projectId: string;
}

/**
 * Inline credentials block (U6) — rendered below a database card when the user
 * clicks "Show credentials". Pops the connection string + user/password/db,
 * each with a copy-to-clipboard button.
 */
function InlineCredentials({
  database,
  showPassword,
  onTogglePassword,
  onCopy,
}: {
  database: ManagedDatabase;
  showPassword: boolean;
  onTogglePassword: () => void;
  onCopy: (text: string, label: string) => void;
}) {
  const creds = database.credentials;
  const internal = database.internal_connection_string;
  const external = database.external_connection_string;

  const maskUrl = (url: string) =>
    showPassword ? url : url.replace(/:[^:@]+@/, ":••••••••@");

  return (
    <div className="relative z-10 mt-3 space-y-2 border-t pt-3 text-xs">
      {creds?.username && (
        <div className="flex items-center justify-between gap-2 px-2 py-1 rounded bg-muted/60">
          <div className="min-w-0 flex-1">
            <div className="text-muted-foreground">Username</div>
            <code className="block truncate">{creds.username}</code>
          </div>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={(e) => {
              e.preventDefault();
              onCopy(creds.username, "Username");
            }}
            title="Copy username"
          >
            <Copy className="h-3 w-3" />
          </Button>
        </div>
      )}
      {creds?.password && (
        <div className="flex items-center justify-between gap-2 px-2 py-1 rounded bg-muted/60">
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-1 text-muted-foreground">
              Password
              <button
                type="button"
                onClick={(e) => {
                  e.preventDefault();
                  onTogglePassword();
                }}
                className="hover:text-foreground"
                title={showPassword ? "Hide password" : "Show password"}
              >
                {showPassword ? (
                  <EyeOff className="h-3 w-3" />
                ) : (
                  <Eye className="h-3 w-3" />
                )}
              </button>
            </div>
            <code className="block truncate">
              {showPassword ? creds.password : "••••••••"}
            </code>
          </div>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={(e) => {
              e.preventDefault();
              onCopy(creds.password, "Password");
            }}
            title="Copy password"
          >
            <Copy className="h-3 w-3" />
          </Button>
        </div>
      )}
      {creds?.database && (
        <div className="flex items-center justify-between gap-2 px-2 py-1 rounded bg-muted/60">
          <div className="min-w-0 flex-1">
            <div className="text-muted-foreground">Database</div>
            <code className="block truncate">{creds.database}</code>
          </div>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={(e) => {
              e.preventDefault();
              onCopy(creds.database!, "Database");
            }}
            title="Copy database name"
          >
            <Copy className="h-3 w-3" />
          </Button>
        </div>
      )}
      {internal && (
        <div className="flex items-center justify-between gap-2 px-2 py-1 rounded bg-muted/60">
          <div className="min-w-0 flex-1">
            <div className="text-muted-foreground">Internal connection</div>
            <code className="block truncate">{maskUrl(internal)}</code>
          </div>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={(e) => {
              e.preventDefault();
              onCopy(internal, "Internal connection string");
            }}
            title="Copy connection string"
          >
            <Copy className="h-3 w-3" />
          </Button>
        </div>
      )}
      {external && (
        <div className="flex items-center justify-between gap-2 px-2 py-1 rounded bg-muted/60">
          <div className="min-w-0 flex-1">
            <div className="text-muted-foreground">External connection</div>
            <code className="block truncate">{maskUrl(external)}</code>
          </div>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={(e) => {
              e.preventDefault();
              onCopy(external, "External connection string");
            }}
            title="Copy connection string"
          >
            <Copy className="h-3 w-3" />
          </Button>
        </div>
      )}
    </div>
  );
}

function DatabaseStatusBadge({ status }: { status: string }) {
  switch (status) {
    case "running":
      return <Badge className="bg-green-500 hover:bg-green-600">Running</Badge>;
    case "stopped":
      return <Badge variant="secondary">Stopped</Badge>;
    case "pending":
      return <Badge variant="outline">Pending</Badge>;
    case "pulling":
      return <Badge className="bg-blue-500 hover:bg-blue-600">Pulling</Badge>;
    case "starting":
      return <Badge className="bg-yellow-500 hover:bg-yellow-600">Starting</Badge>;
    case "failed":
      return <Badge variant="destructive">Failed</Badge>;
    default:
      return <Badge variant="outline">{status}</Badge>;
  }
}

export function DatabasesTab({ project, projectId }: DatabasesTabProps) {
  const queryClient = useQueryClient();
  const { currentTeamId } = useTeamContext();
  const [isCreateDbDialogOpen, setIsCreateDbDialogOpen] = useState(false);
  const [isDeleteDbDialogOpen, setIsDeleteDbDialogOpen] = useState(false);
  const [isCredentialsDialogOpen, setIsCredentialsDialogOpen] = useState(false);
  const [selectedDatabase, setSelectedDatabase] = useState<ManagedDatabase | null>(null);
  const [selectedDbType, setSelectedDbType] = useState<DatabaseType>("postgres");
  const [showCustomCredentials, setShowCustomCredentials] = useState(false);
  const [showPasswords, setShowPasswords] = useState(false);
  const [revealedDatabase, setRevealedDatabase] = useState<ManagedDatabase | null>(null);
  // Inline credentials state for U6: dbId → revealed ManagedDatabase, and a
  // separate set of dbIds whose password should be shown as plaintext.
  const [inlineCredentials, setInlineCredentials] = useState<
    Record<string, ManagedDatabase>
  >({});
  const [inlinePasswordRevealed, setInlinePasswordRevealed] = useState<
    Set<string>
  >(new Set());
  const [inlineLoadingId, setInlineLoadingId] = useState<string | null>(null);

  // Form state
  const [dbName, setDbName] = useState("");
  const [dbVersion, setDbVersion] = useState("latest");
  const [dbPublicAccess, setDbPublicAccess] = useState(false);
  const [dbUsername, setDbUsername] = useState("");
  const [dbPassword, setDbPassword] = useState("");
  const [dbDatabase, setDbDatabase] = useState("");
  const [dbRootPassword, setDbRootPassword] = useState("");
  const [dbCustomImage, setDbCustomImage] = useState("");
  const [dbInitCommands, setDbInitCommands] = useState("");
  const [showAdvancedOptions, setShowAdvancedOptions] = useState(false);

  const dbTypeConfig = DATABASE_TYPES.find((t) => t.type === selectedDbType);

  const resetDbForm = () => {
    setDbName("");
    setDbVersion("latest");
    setDbPublicAccess(false);
    setDbUsername("");
    setDbPassword("");
    setDbDatabase("");
    setDbRootPassword("");
    setDbCustomImage("");
    setDbInitCommands("");
    setSelectedDbType("postgres");
    setShowCustomCredentials(false);
    setShowAdvancedOptions(false);
  };

  const copyToClipboard = (text: string, label: string) => {
    navigator.clipboard.writeText(text);
    toast.success(`${label} copied to clipboard`);
  };

  const handleViewCredentials = async (database: ManagedDatabase) => {
    try {
      const revealed = await api.getDatabase(database.id, true);
      setRevealedDatabase(revealed);
      setIsCredentialsDialogOpen(true);
    } catch {
      toast.error("Failed to fetch credentials");
    }
  };

  // U6: toggle inline credentials display for a database card.
  const toggleInlineCredentials = async (database: ManagedDatabase) => {
    if (inlineCredentials[database.id]) {
      // Already shown — hide.
      setInlineCredentials((prev) => {
        const next = { ...prev };
        delete next[database.id];
        return next;
      });
      setInlinePasswordRevealed((prev) => {
        const next = new Set(prev);
        next.delete(database.id);
        return next;
      });
      return;
    }
    setInlineLoadingId(database.id);
    try {
      const revealed = await api.getDatabase(database.id, true);
      setInlineCredentials((prev) => ({ ...prev, [database.id]: revealed }));
    } catch {
      toast.error("Failed to fetch credentials");
    } finally {
      setInlineLoadingId(null);
    }
  };

  const toggleInlinePassword = (databaseId: string) => {
    setInlinePasswordRevealed((prev) => {
      const next = new Set(prev);
      if (next.has(databaseId)) {
        next.delete(databaseId);
      } else {
        next.add(databaseId);
      }
      return next;
    });
  };

  const createDatabaseMutation = useMutation({
    mutationFn: async () => {
      if (!dbName.trim()) {
        throw new Error("Database name is required");
      }
      // Serialize init commands: one per line → JSON array
      const initCommandLines = dbInitCommands
        .split("\n")
        .map((l) => l.trim())
        .filter((l) => l.length > 0);

      return api.createDatabase({
        name: dbName.trim(),
        db_type: selectedDbType,
        version: dbVersion,
        public_access: dbPublicAccess,
        project_id: projectId,
        team_id: currentTeamId ?? undefined,
        ...(dbUsername.trim() ? { username: dbUsername.trim() } : {}),
        ...(dbPassword.trim() ? { password: dbPassword.trim() } : {}),
        ...(dbDatabase.trim() ? { database: dbDatabase.trim() } : {}),
        ...(dbRootPassword.trim() ? { root_password: dbRootPassword.trim() } : {}),
        ...(dbCustomImage.trim() ? { custom_image: dbCustomImage.trim() } : {}),
        ...(initCommandLines.length > 0
          ? { init_commands: JSON.stringify(initCommandLines) }
          : {}),
      });
    },
    onSuccess: () => {
      toast.success("Database created");
      setIsCreateDbDialogOpen(false);
      resetDbForm();
      queryClient.invalidateQueries({ queryKey: ["project", projectId] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const deleteDatabaseMutation = useMutation({
    mutationFn: (databaseId: string) => api.deleteDatabase(databaseId),
    onSuccess: () => {
      toast.success("Database deleted");
      setIsDeleteDbDialogOpen(false);
      setSelectedDatabase(null);
      queryClient.invalidateQueries({ queryKey: ["project", projectId] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const startDatabaseMutation = useMutation({
    mutationFn: (databaseId: string) => api.startDatabase(databaseId),
    onSuccess: () => {
      toast.success("Database starting");
      queryClient.invalidateQueries({ queryKey: ["project", projectId] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const stopDatabaseMutation = useMutation({
    mutationFn: (databaseId: string) => api.stopDatabase(databaseId),
    onSuccess: () => {
      toast.success("Database stopped");
      queryClient.invalidateQueries({ queryKey: ["project", projectId] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  return (
    <>
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Databases</CardTitle>
          <Button onClick={() => setIsCreateDbDialogOpen(true)}>
            <Database className="mr-2 h-4 w-4" />
            Create Database
          </Button>
        </CardHeader>
        <CardContent>
          {!project.databases || project.databases.length === 0 ? (
            <div className="py-8 text-center">
              <Database className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
              <p className="text-muted-foreground">
                No databases in this project yet. Use the
                {" "}<span className="font-medium">Create Database</span>{" "}
                button above to add one.
              </p>
            </div>
          ) : (
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              {project.databases.map((db) => {
                const dbTypeInfo = DATABASE_TYPES.find((t) => t.type === db.db_type);
                return (
                  <Card
                    key={db.id}
                    className="group relative hover:shadow-md transition-shadow"
                  >
                    <Link to={`/databases/${db.id}`} className="absolute inset-0 z-0" />
                    <CardHeader className="pb-2">
                      <div className="flex items-start justify-between">
                        <div className="space-y-1">
                          <div className="flex items-center gap-2">
                            <CardTitle className="text-base font-semibold">
                              {db.name}
                            </CardTitle>
                            {db.status === "failed" && db.error_message && (
                              <TooltipProvider>
                                <Tooltip>
                                  <TooltipTrigger>
                                    <AlertCircle className="h-4 w-4 text-destructive" />
                                  </TooltipTrigger>
                                  <TooltipContent className="max-w-xs">
                                    <p className="text-sm">{db.error_message}</p>
                                  </TooltipContent>
                                </Tooltip>
                              </TooltipProvider>
                            )}
                          </div>
                          <div className="flex items-center gap-2">
                            <DatabaseStatusBadge status={db.status} />
                            <Badge variant="outline" className="capitalize text-xs">
                              {dbTypeInfo?.name || db.db_type} {db.version}
                            </Badge>
                          </div>
                        </div>
                        <div className="flex items-center gap-1 relative z-10 opacity-0 group-hover:opacity-100 transition-opacity">
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-7 w-7"
                            title="View Credentials"
                            onClick={(e) => {
                              e.preventDefault();
                              handleViewCredentials(db);
                            }}
                          >
                            <Eye className="h-3.5 w-3.5" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-7 w-7 text-destructive"
                            title="Delete Database"
                            onClick={(e) => {
                              e.preventDefault();
                              setSelectedDatabase(db);
                              setIsDeleteDbDialogOpen(true);
                            }}
                          >
                            <Trash2 className="h-3.5 w-3.5" />
                          </Button>
                        </div>
                      </div>
                    </CardHeader>
                    <CardContent className="pt-0 pb-4">
                      <div className="flex items-center justify-between text-sm">
                        <span className="text-muted-foreground">
                          {db.public_access && db.external_port > 0 ? (
                            <span className="font-mono">Port {db.external_port}</span>
                          ) : (
                            "Internal only"
                          )}
                        </span>
                        <div className="relative z-10 flex items-center gap-1">
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-7 px-2"
                            disabled={inlineLoadingId === db.id}
                            onClick={(e) => {
                              e.preventDefault();
                              toggleInlineCredentials(db);
                            }}
                            title={
                              inlineCredentials[db.id]
                                ? "Hide credentials"
                                : "Show credentials"
                            }
                          >
                            {inlineCredentials[db.id] ? (
                              <EyeOff className="h-3 w-3 mr-1" />
                            ) : (
                              <Eye className="h-3 w-3 mr-1" />
                            )}
                            {inlineLoadingId === db.id
                              ? "Loading…"
                              : inlineCredentials[db.id]
                              ? "Hide credentials"
                              : "Show credentials"}
                          </Button>
                          {db.status === "stopped" && (
                            <Button
                              variant="outline"
                              size="sm"
                              className="h-7 px-2"
                              disabled={startDatabaseMutation.isPending}
                              onClick={(e) => {
                                e.preventDefault();
                                startDatabaseMutation.mutate(db.id);
                              }}
                            >
                              <Play className="h-3 w-3 mr-1" />
                              Start
                            </Button>
                          )}
                          {db.status === "running" && (
                            <Button
                              variant="outline"
                              size="sm"
                              className="h-7 px-2"
                              disabled={stopDatabaseMutation.isPending}
                              onClick={(e) => {
                                e.preventDefault();
                                stopDatabaseMutation.mutate(db.id);
                              }}
                            >
                              <Square className="h-3 w-3 mr-1" />
                              Stop
                            </Button>
                          )}
                        </div>
                      </div>
                      {inlineCredentials[db.id] && (
                        <InlineCredentials
                          database={inlineCredentials[db.id]}
                          showPassword={inlinePasswordRevealed.has(db.id)}
                          onTogglePassword={() => toggleInlinePassword(db.id)}
                          onCopy={copyToClipboard}
                        />
                      )}
                    </CardContent>
                  </Card>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Create Database Dialog */}
      <Dialog
        open={isCreateDbDialogOpen}
        onOpenChange={(open) => {
          setIsCreateDbDialogOpen(open);
          if (!open) resetDbForm();
        }}
      >
        <DialogContent className="max-w-lg">
          <form
            onSubmit={(e) => {
              e.preventDefault();
              createDatabaseMutation.mutate();
            }}
          >
            <DialogHeader>
              <DialogTitle>Create Database</DialogTitle>
              <DialogDescription>
                Deploy a new managed database with auto-generated credentials.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label htmlFor="db-name">Name</Label>
                <Input
                  id="db-name"
                  value={dbName}
                  onChange={(e) => setDbName(e.target.value)}
                  placeholder="e.g., my-postgres-db"
                  pattern="[a-zA-Z0-9-]+"
                  title="Only alphanumeric characters and hyphens are allowed"
                  required
                />
                <p className="text-xs text-muted-foreground">
                  Only letters, numbers, and hyphens allowed
                </p>
              </div>

              <div className="space-y-2">
                <Label>Database Type</Label>
                <div className="grid grid-cols-2 gap-2">
                  {DATABASE_TYPES.map((config) => (
                    <button
                      key={config.type}
                      type="button"
                      className={`p-3 border rounded-lg text-left transition-colors ${
                        selectedDbType === config.type
                          ? "border-primary bg-primary/5"
                          : "border-border hover:border-primary/50"
                      }`}
                      onClick={() => setSelectedDbType(config.type)}
                    >
                      <div className="font-medium">{config.name}</div>
                      <div className="text-xs text-muted-foreground">
                        Port {config.defaultPort}
                      </div>
                    </button>
                  ))}
                </div>
              </div>

              <div className="space-y-2">
                <Label htmlFor="db-version">Version</Label>
                <Select value={dbVersion} onValueChange={setDbVersion}>
                  <SelectTrigger>
                    <SelectValue placeholder="Select version" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="latest">
                      Latest ({dbTypeConfig?.defaultVersion})
                    </SelectItem>
                    {dbTypeConfig?.versions.map((v) => (
                      <SelectItem key={v} value={v}>
                        {v}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              <div className="flex items-center space-x-2">
                <Checkbox
                  id="public_access"
                  checked={dbPublicAccess}
                  onCheckedChange={(checked) => setDbPublicAccess(checked === true)}
                />
                <Label htmlFor="public_access" className="text-sm font-normal">
                  Enable public access (expose port to host)
                </Label>
              </div>

              <Collapsible open={showCustomCredentials} onOpenChange={setShowCustomCredentials}>
                <CollapsibleTrigger asChild>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="flex items-center gap-1 p-0 h-auto hover:bg-transparent"
                  >
                    <ChevronDown
                      className={`h-4 w-4 transition-transform ${
                        showCustomCredentials ? "rotate-180" : ""
                      }`}
                    />
                    <span className="text-sm text-muted-foreground">
                      Custom credentials (optional)
                    </span>
                  </Button>
                </CollapsibleTrigger>
                <CollapsibleContent className="space-y-3 pt-3">
                  <p className="text-xs text-muted-foreground">
                    Leave fields empty to auto-generate secure credentials.
                  </p>
                  <div className="space-y-2">
                    <Label htmlFor="db-username">Username</Label>
                    <Input
                      id="db-username"
                      value={dbUsername}
                      onChange={(e) => setDbUsername(e.target.value)}
                      placeholder="Auto-generated if empty"
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="db-password">Password</Label>
                    <Input
                      id="db-password"
                      value={dbPassword}
                      onChange={(e) => setDbPassword(e.target.value)}
                      type="password"
                      placeholder="Auto-generated if empty"
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="db-database">Database Name</Label>
                    <Input
                      id="db-database"
                      value={dbDatabase}
                      onChange={(e) => setDbDatabase(e.target.value)}
                      placeholder="Defaults to username"
                    />
                  </div>
                  {(selectedDbType === "mysql" || selectedDbType === "mariadb") && (
                    <div className="space-y-2">
                      <Label htmlFor="db-root-password">Root Password</Label>
                      <Input
                        id="db-root-password"
                        value={dbRootPassword}
                        onChange={(e) => setDbRootPassword(e.target.value)}
                        type="password"
                        placeholder="Auto-generated if empty"
                      />
                      <p className="text-xs text-muted-foreground">
                        MySQL root password for administrative access
                      </p>
                    </div>
                  )}
                </CollapsibleContent>
              </Collapsible>

              <Collapsible open={showAdvancedOptions} onOpenChange={setShowAdvancedOptions}>
                <CollapsibleTrigger asChild>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="flex items-center gap-1 p-0 h-auto hover:bg-transparent"
                  >
                    <ChevronDown
                      className={`h-4 w-4 transition-transform ${
                        showAdvancedOptions ? "rotate-180" : ""
                      }`}
                    />
                    <span className="text-sm text-muted-foreground">
                      Advanced options (custom image &amp; init commands)
                    </span>
                  </Button>
                </CollapsibleTrigger>
                <CollapsibleContent className="space-y-3 pt-3">
                  <div className="space-y-2">
                    <Label htmlFor="db-custom-image">Custom Docker Image</Label>
                    <Input
                      id="db-custom-image"
                      value={dbCustomImage}
                      onChange={(e) => setDbCustomImage(e.target.value)}
                      placeholder={`Default: ${selectedDbType}:${dbVersion}`}
                      className="font-mono text-sm"
                    />
                    <p className="text-xs text-muted-foreground">
                      Override the default image, e.g.{" "}
                      <code className="bg-muted px-1 rounded">
                        timescaledb/timescaledb-ha:pg16-latest
                      </code>
                    </p>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="db-init-commands">Init SQL Commands</Label>
                    <Textarea
                      id="db-init-commands"
                      value={dbInitCommands}
                      onChange={(e) => setDbInitCommands(e.target.value)}
                      placeholder={"CREATE EXTENSION IF NOT EXISTS postgis;\nCREATE SCHEMA app;"}
                      className="font-mono text-sm min-h-[80px]"
                    />
                    <p className="text-xs text-muted-foreground">
                      One SQL command per line. Run after the database first starts.
                    </p>
                  </div>
                </CollapsibleContent>
              </Collapsible>
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => setIsCreateDbDialogOpen(false)}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={createDatabaseMutation.isPending}>
                {createDatabaseMutation.isPending ? "Creating..." : "Create Database"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Database Credentials Dialog */}
      <Dialog open={isCredentialsDialogOpen} onOpenChange={setIsCredentialsDialogOpen}>
        <DialogContent className="max-w-xl">
          <DialogHeader>
            <DialogTitle>Database Credentials</DialogTitle>
            <DialogDescription>
              Connection details for {revealedDatabase?.name}
            </DialogDescription>
          </DialogHeader>
          {revealedDatabase && (
            <div className="space-y-4 py-4">
              <div className="flex items-center justify-end">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setShowPasswords(!showPasswords)}
                >
                  {showPasswords ? (
                    <>
                      <EyeOff className="h-4 w-4 mr-2" /> Hide Passwords
                    </>
                  ) : (
                    <>
                      <Eye className="h-4 w-4 mr-2" /> Show Passwords
                    </>
                  )}
                </Button>
              </div>

              <div className="space-y-3">
                {revealedDatabase.credentials?.username && (
                  <div className="flex items-center justify-between p-2 bg-muted rounded">
                    <div>
                      <div className="text-xs text-muted-foreground">Username</div>
                      <code className="text-sm">{revealedDatabase.credentials.username}</code>
                    </div>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() =>
                        copyToClipboard(revealedDatabase.credentials!.username, "Username")
                      }
                    >
                      <Copy className="h-4 w-4" />
                    </Button>
                  </div>
                )}

                {revealedDatabase.credentials?.password && (
                  <div className="flex items-center justify-between p-2 bg-muted rounded">
                    <div>
                      <div className="text-xs text-muted-foreground">Password</div>
                      <code className="text-sm">
                        {showPasswords
                          ? revealedDatabase.credentials.password
                          : "----------------"}
                      </code>
                    </div>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() =>
                        copyToClipboard(revealedDatabase.credentials!.password, "Password")
                      }
                    >
                      <Copy className="h-4 w-4" />
                    </Button>
                  </div>
                )}

                {revealedDatabase.credentials?.database && (
                  <div className="flex items-center justify-between p-2 bg-muted rounded">
                    <div>
                      <div className="text-xs text-muted-foreground">Database</div>
                      <code className="text-sm">{revealedDatabase.credentials.database}</code>
                    </div>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() =>
                        copyToClipboard(
                          revealedDatabase.credentials!.database!,
                          "Database"
                        )
                      }
                    >
                      <Copy className="h-4 w-4" />
                    </Button>
                  </div>
                )}

                {revealedDatabase.internal_connection_string && (
                  <div className="p-2 bg-muted rounded">
                    <div className="flex items-center justify-between mb-1">
                      <div className="text-xs text-muted-foreground">
                        Internal Connection String
                      </div>
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() =>
                          copyToClipboard(
                            revealedDatabase.internal_connection_string!,
                            "Internal connection string"
                          )
                        }
                      >
                        <Copy className="h-4 w-4" />
                      </Button>
                    </div>
                    <code className="text-xs break-all">
                      {showPasswords
                        ? revealedDatabase.internal_connection_string
                        : revealedDatabase.internal_connection_string.replace(
                            /:[^:@]+@/,
                            ":--------@"
                          )}
                    </code>
                  </div>
                )}

                {revealedDatabase.external_connection_string && (
                  <div className="p-2 bg-muted rounded">
                    <div className="flex items-center justify-between mb-1">
                      <div className="text-xs text-muted-foreground">
                        External Connection String
                      </div>
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() =>
                          copyToClipboard(
                            revealedDatabase.external_connection_string!,
                            "External connection string"
                          )
                        }
                      >
                        <Copy className="h-4 w-4" />
                      </Button>
                    </div>
                    <code className="text-xs break-all">
                      {showPasswords
                        ? revealedDatabase.external_connection_string
                        : revealedDatabase.external_connection_string.replace(
                            /:[^:@]+@/,
                            ":--------@"
                          )}
                    </code>
                  </div>
                )}
              </div>
            </div>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => setIsCredentialsDialogOpen(false)}>
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Database Dialog */}
      <Dialog open={isDeleteDbDialogOpen} onOpenChange={setIsDeleteDbDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Database</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete "{selectedDatabase?.name}"? This will stop
              the container and delete all data. This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setIsDeleteDbDialogOpen(false);
                setSelectedDatabase(null);
              }}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              disabled={deleteDatabaseMutation.isPending}
              onClick={() => {
                if (selectedDatabase) {
                  deleteDatabaseMutation.mutate(selectedDatabase.id);
                }
              }}
            >
              {deleteDatabaseMutation.isPending ? "Deleting..." : "Delete Database"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
