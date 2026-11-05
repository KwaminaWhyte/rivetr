/**
 * "From secret manager": pick a connection, browse its paths and keys (names
 * only, values never reach the browser) and either add a variable whose value
 * is a `secret://` reference, or import every key at a path.
 */
import { useState } from "react";
import { useMutation, useQuery, useQueryClient, type QueryKey } from "@tanstack/react-query";
import { Link } from "react-router";
import { toast } from "sonner";
import { ChevronRight, Folder, KeyRound, Loader2, Lock } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
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
import { useTeamContext } from "@/lib/team-context";
import {
  PROVIDER_LABELS,
  formatSecretReference,
  parseSecretReference,
  secretManagersApi,
  type SecretOwnerType,
} from "@/lib/api/secret-managers";

const ENV_KEY_RE = /^[A-Za-z_][A-Za-z0-9_]*$/;

/** A `secret://` value shown as a chip instead of plain text. */
export function SecretRefChip({ value }: { value: string }) {
  const ref = parseSecretReference(value);
  if (!ref) return null;
  return (
    <Badge
      variant="secondary"
      className="max-w-full gap-1 font-mono text-xs font-normal"
      title={`Resolved at deploy time from secret manager connection "${ref.connection}"`}
    >
      <Lock className="h-3 w-3 shrink-0" />
      <span className="truncate">
        {ref.connection}
        {ref.path ? `/${ref.path}` : ""}
        {ref.key ? `#${ref.key}` : ""}
      </span>
    </Badge>
  );
}

/** Does this value reference a secret manager? */
export function isSecretRefValue(value: string | null | undefined): boolean {
  return parseSecretReference(value) !== null;
}

interface PickerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** "variable": add one key as a variable; "import": link every key at a path. */
  mode: "variable" | "import";
  ownerType: SecretOwnerType;
  ownerId: string;
  /** Create a variable with this value (mode "variable"). */
  createVariable?: (key: string, value: string) => Promise<unknown>;
  /** Queries to refresh after a change. */
  invalidate?: QueryKey[];
}

export function SecretPickerDialog({
  open,
  onOpenChange,
  mode,
  ownerType,
  ownerId,
  createVariable,
  invalidate = [],
}: PickerProps) {
  const qc = useQueryClient();
  const { currentTeamId } = useTeamContext();
  const teamId = currentTeamId ?? undefined;
  const [connectionId, setConnectionId] = useState<string>("");
  const [path, setPath] = useState("");
  const [key, setKey] = useState<string | null>(null);
  const [varName, setVarName] = useState("");
  const [prefix, setPrefix] = useState("");

  const { data: connections = [], isLoading: loadingConnections } = useQuery({
    queryKey: ["secret-connections", teamId],
    queryFn: () => secretManagersApi.list(teamId),
    enabled: open,
  });
  const connection = connections.find((c) => c.id === connectionId);

  const {
    data: listing,
    isFetching: browsing,
    error: browseError,
  } = useQuery({
    queryKey: ["secret-browse", connectionId, path],
    queryFn: () => secretManagersApi.browse(connectionId, path),
    enabled: open && !!connectionId,
    retry: false,
  });

  const reset = () => {
    setPath("");
    setKey(null);
    setVarName("");
    setPrefix("");
  };

  const done = () => {
    for (const k of invalidate) qc.invalidateQueries({ queryKey: k });
    onOpenChange(false);
    reset();
  };

  const addVariable = useMutation({
    mutationFn: () => {
      if (!connection || !createVariable) throw new Error("Pick a connection");
      return createVariable(varName, formatSecretReference(connection.name, path, key));
    },
    onSuccess: () => {
      toast.success(`${varName} now references ${connection?.name}`);
      done();
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const addImport = useMutation({
    mutationFn: () =>
      secretManagersApi.createImport(ownerType, ownerId, {
        connection_id: connectionId,
        path,
        prefix,
      }),
    onSuccess: () => {
      toast.success("Path imported. Keys are added on the next deploy.");
      qc.invalidateQueries({ queryKey: ["secret-imports", ownerType, ownerId] });
      done();
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const segments = path ? path.split("/") : [];
  const reference = connection ? formatSecretReference(connection.name, path, key) : "";
  const awsWholeSecret =
    connection?.provider === "aws_secrets_manager" && !!path && listing?.keys.length === 0;

  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        onOpenChange(o);
        if (!o) reset();
      }}
    >
      <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>
            {mode === "variable" ? "Variable from secret manager" : "Import a secret path"}
          </DialogTitle>
          <DialogDescription>
            {mode === "variable"
              ? "The variable stores a reference. Rivetr fetches the value at deploy time; it is never shown here."
              : "Every key at the path becomes a variable (with the optional prefix) at deploy time. Variables you define yourself win."}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="space-y-2">
            <Label>Connection</Label>
            {!loadingConnections && connections.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                No connections yet.{" "}
                <Link to="/integrations/secrets" className="underline">
                  Add one under Integrations
                </Link>
                .
              </p>
            ) : (
              <Select
                value={connectionId}
                onValueChange={(v) => {
                  setConnectionId(v);
                  reset();
                }}
              >
                <SelectTrigger className="w-full">
                  <SelectValue placeholder="Choose a connection" />
                </SelectTrigger>
                <SelectContent>
                  {connections.map((c) => (
                    <SelectItem key={c.id} value={c.id}>
                      {c.name} ({PROVIDER_LABELS[c.provider] ?? c.provider})
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </div>

          {connection && (
            <div className="space-y-2">
              <Label htmlFor="secret-path">Path</Label>
              <div className="flex flex-wrap items-center gap-1 text-sm">
                <button
                  type="button"
                  className="text-primary hover:underline"
                  onClick={() => {
                    setPath("");
                    setKey(null);
                  }}
                >
                  {connection.name}
                </button>
                {segments.map((s, i) => (
                  <span key={i} className="flex items-center gap-1">
                    <ChevronRight className="h-3 w-3 text-muted-foreground" />
                    <button
                      type="button"
                      className="text-primary hover:underline"
                      onClick={() => {
                        setPath(segments.slice(0, i + 1).join("/"));
                        setKey(null);
                      }}
                    >
                      {s}
                    </button>
                  </span>
                ))}
              </div>
              <Input
                id="secret-path"
                value={path}
                placeholder="Type a path if browsing is not allowed"
                onChange={(e) => {
                  setPath(e.target.value.replace(/^\/+/, ""));
                  setKey(null);
                }}
              />
              <div className="rounded-md border max-h-60 overflow-y-auto divide-y">
                {browsing ? (
                  <div className="flex items-center gap-2 p-3 text-sm text-muted-foreground">
                    <Loader2 className="h-4 w-4 animate-spin" /> Loading
                  </div>
                ) : browseError ? (
                  <p className="p-3 text-sm text-destructive">
                    {browseError instanceof Error ? browseError.message : "Could not browse"}
                  </p>
                ) : (
                  <>
                    {listing?.entries.map((e) => (
                      <button
                        key={e.path}
                        type="button"
                        className="flex w-full items-center gap-2 p-2 text-left text-sm hover:bg-muted"
                        onClick={() => {
                          setPath(e.path);
                          setKey(null);
                        }}
                      >
                        {e.kind === "folder" ? (
                          <Folder className="h-4 w-4 text-muted-foreground" />
                        ) : (
                          <Lock className="h-4 w-4 text-muted-foreground" />
                        )}
                        {e.name}
                      </button>
                    ))}
                    {mode === "variable" &&
                      listing?.keys.map((k) => (
                        <button
                          key={k}
                          type="button"
                          className={`flex w-full items-center gap-2 p-2 text-left font-mono text-sm hover:bg-muted ${
                            key === k ? "bg-muted font-semibold" : ""
                          }`}
                          onClick={() => {
                            setKey(k);
                            if (!varName || varName === key) {
                              setVarName(ENV_KEY_RE.test(k) ? k : k.toUpperCase().replace(/[^A-Z0-9_]/g, "_"));
                            }
                          }}
                        >
                          <KeyRound className="h-4 w-4 text-muted-foreground" />
                          {k}
                        </button>
                      ))}
                    {mode === "import" && listing && listing.keys.length > 0 && (
                      <p className="p-2 text-xs text-muted-foreground">
                        {listing.keys.length} key(s) here: {listing.keys.slice(0, 8).join(", ")}
                        {listing.keys.length > 8 ? ", ..." : ""}
                      </p>
                    )}
                    {listing && listing.entries.length === 0 && listing.keys.length === 0 && (
                      <p className="p-3 text-sm text-muted-foreground">
                        {awsWholeSecret
                          ? "Plain-text secret: add it without a key to use the whole value."
                          : "Nothing to browse here."}
                      </p>
                    )}
                  </>
                )}
              </div>
            </div>
          )}

          {connection && mode === "variable" && (
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="secret-key">Key</Label>
                <Input
                  id="secret-key"
                  value={key ?? ""}
                  placeholder={awsWholeSecret ? "Whole secret" : "Pick or type a key"}
                  onChange={(e) => setKey(e.target.value || null)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="secret-var">Variable name</Label>
                <Input
                  id="secret-var"
                  value={varName}
                  onChange={(e) => setVarName(e.target.value.toUpperCase())}
                  placeholder="DATABASE_URL"
                />
              </div>
              <p className="sm:col-span-2 break-all font-mono text-xs text-muted-foreground">
                {reference}
              </p>
            </div>
          )}

          {connection && mode === "import" && (
            <div className="space-y-2">
              <Label htmlFor="secret-prefix">Key prefix (optional)</Label>
              <Input
                id="secret-prefix"
                value={prefix}
                placeholder="APP_"
                onChange={(e) => setPrefix(e.target.value.toUpperCase())}
              />
            </div>
          )}
        </div>

        <DialogFooter>
          {mode === "variable" ? (
            <Button
              disabled={
                !connection ||
                !ENV_KEY_RE.test(varName) ||
                (!key && connection.provider !== "aws_secrets_manager") ||
                addVariable.isPending
              }
              onClick={() => addVariable.mutate()}
            >
              {addVariable.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              Add variable
            </Button>
          ) : (
            <Button
              disabled={!connection || (!!prefix && !ENV_KEY_RE.test(prefix)) || addImport.isPending}
              onClick={() => addImport.mutate()}
            >
              {addImport.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              Import path
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/** Button + dialog for an env var editor toolbar. */
export function FromSecretManagerButton(
  props: Omit<PickerProps, "open" | "onOpenChange" | "mode">,
) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button variant="outline" size="sm" className="gap-2" onClick={() => setOpen(true)}>
        <Lock className="h-4 w-4" />
        From secret manager
      </Button>
      <SecretPickerDialog {...props} mode="variable" open={open} onOpenChange={setOpen} />
    </>
  );
}
