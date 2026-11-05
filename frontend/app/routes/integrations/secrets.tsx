import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { KeyRound, Loader2, Pencil, PlugZap, Plus, Trash2 } from "lucide-react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { QueryError } from "@/components/query-error";
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
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { useTeamContext } from "@/lib/team-context";
import {
  PROVIDER_LABELS,
  secretManagersApi,
  type SecretConnection,
  type SecretConnectionConfig,
  type SecretConnectionCredentials,
  type SecretProviderKind,
  type SecretTestResult,
} from "@/lib/api/secret-managers";

export function meta() {
  return [
    { title: "Secret managers - Rivetr" },
    {
      name: "description",
      content: "Connect Vault, OpenBao, Infisical, Doppler or AWS Secrets Manager and reference secrets from environment variables",
    },
  ];
}

type ConfigKey = keyof SecretConnectionConfig;
type CredKey = keyof SecretConnectionCredentials;

interface FieldSpec {
  key: ConfigKey | CredKey;
  label: string;
  secret?: boolean;
  required?: boolean;
  placeholder?: string;
  help?: string;
  /** Only shown for this auth method. */
  auth?: string;
}

const AUTH_METHODS: Partial<Record<SecretProviderKind, { value: string; label: string }[]>> = {
  vault: [
    { value: "token", label: "Token" },
    { value: "approle", label: "AppRole" },
  ],
  openbao: [
    { value: "token", label: "Token" },
    { value: "approle", label: "AppRole" },
  ],
  aws_secrets_manager: [
    { value: "access_key", label: "Access key" },
    { value: "instance_role", label: "EC2 instance role" },
  ],
};

const vaultFields: FieldSpec[] = [
  { key: "address", label: "Address", required: true, placeholder: "https://vault.example.com:8200" },
  { key: "mount", label: "KV v2 mount", placeholder: "secret" },
  { key: "namespace", label: "Namespace", placeholder: "Optional (Vault Enterprise / OpenBao)" },
  { key: "token", label: "Token", secret: true, required: true, auth: "token" },
  { key: "role_id", label: "Role ID", required: true, auth: "approle" },
  { key: "secret_id", label: "Secret ID", secret: true, required: true, auth: "approle" },
  { key: "approle_mount", label: "AppRole mount", placeholder: "approle", auth: "approle" },
];

const FIELDS: Record<SecretProviderKind, FieldSpec[]> = {
  vault: vaultFields,
  openbao: vaultFields,
  infisical: [
    { key: "site_url", label: "Site URL", placeholder: "https://app.infisical.com" },
    { key: "project_id", label: "Project ID", required: true },
    { key: "environment", label: "Environment slug", required: true, placeholder: "prod" },
    { key: "client_id", label: "Machine identity client ID", required: true },
    { key: "client_secret", label: "Machine identity client secret", secret: true, required: true },
  ],
  doppler: [
    { key: "token", label: "Service token", secret: true, required: true, placeholder: "dp.st.prd...." },
    { key: "project", label: "Project", help: "Optional: a service token is already scoped to one config" },
    { key: "config", label: "Config" },
    { key: "api_url", label: "API URL", placeholder: "https://api.doppler.com" },
  ],
  aws_secrets_manager: [
    { key: "region", label: "Region", required: true, placeholder: "eu-west-1" },
    { key: "access_key_id", label: "Access key ID", required: true, auth: "access_key" },
    { key: "secret_access_key", label: "Secret access key", secret: true, required: true, auth: "access_key" },
    { key: "session_token", label: "Session token", secret: true, auth: "access_key", help: "Only for temporary credentials" },
    { key: "endpoint", label: "Endpoint override", placeholder: "Optional (VPC endpoint)" },
  ],
};

const PATH_HINT: Record<SecretProviderKind, string> = {
  vault: "secret://NAME/path/to/secret#KEY",
  openbao: "secret://NAME/path/to/secret#KEY",
  infisical: "secret://NAME/folder#KEY",
  doppler: "secret://NAME/#KEY",
  aws_secrets_manager: "secret://NAME/secret-name#json_key",
};

const CRED_KEYS: CredKey[] = ["token", "secret_id", "client_secret", "secret_access_key", "session_token"];

function isCred(k: string): k is CredKey {
  return (CRED_KEYS as string[]).includes(k);
}

interface FormState {
  name: string;
  provider: SecretProviderKind;
  values: Record<string, string>;
}

const emptyForm = (): FormState => ({ name: "", provider: "vault", values: {} });

function formFrom(c: SecretConnection): FormState {
  const values: Record<string, string> = {};
  for (const [k, v] of Object.entries(c.config)) {
    if (typeof v === "string") values[k] = v;
  }
  return { name: c.name, provider: c.provider, values };
}

function toPayload(form: FormState): {
  config: SecretConnectionConfig;
  credentials: SecretConnectionCredentials;
} {
  const config: SecretConnectionConfig = {};
  const credentials: SecretConnectionCredentials = {};
  const auth = form.values.auth_method;
  if (auth) config.auth_method = auth;
  for (const f of FIELDS[form.provider]) {
    if (f.auth && auth && f.auth !== auth) continue;
    const v = (form.values[f.key] ?? "").trim();
    if (!v) continue;
    if (isCred(f.key)) credentials[f.key] = v;
    else (config as Record<string, string>)[f.key] = v;
  }
  return { config, credentials };
}

function TestBadge({ c }: { c: SecretConnection }) {
  if (c.last_test_ok === null) return <Badge variant="outline">Not tested</Badge>;
  return c.last_test_ok ? (
    <Badge className="bg-green-600" title={c.last_test_message ?? ""}>Connected</Badge>
  ) : (
    <Badge variant="destructive" title={c.last_test_message ?? ""}>Failed</Badge>
  );
}

export default function SecretManagersPage() {
  const qc = useQueryClient();
  const { currentTeamId } = useTeamContext();
  const teamId = currentTeamId ?? undefined;

  const { data: connections = [], isLoading, error, refetch } = useQuery({
    queryKey: ["secret-connections", teamId],
    queryFn: () => secretManagersApi.list(teamId),
  });

  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<SecretConnection | null>(null);
  const [form, setForm] = useState<FormState>(emptyForm);
  const [testResult, setTestResult] = useState<SecretTestResult | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<SecretConnection | null>(null);

  const invalidate = () => qc.invalidateQueries({ queryKey: ["secret-connections"] });

  const openCreate = () => {
    setEditing(null);
    setForm(emptyForm());
    setTestResult(null);
    setOpen(true);
  };
  const openEdit = (c: SecretConnection) => {
    setEditing(c);
    setForm(formFrom(c));
    setTestResult(null);
    setOpen(true);
  };

  const save = useMutation({
    mutationFn: () => {
      const { config, credentials } = toPayload(form);
      return editing
        ? secretManagersApi.update(editing.id, { name: form.name, config, credentials })
        : secretManagersApi.create({
            team_id: teamId ?? null,
            name: form.name,
            provider: form.provider,
            config,
            credentials,
          });
    },
    onSuccess: () => {
      invalidate();
      toast.success(editing ? "Connection updated" : "Connection added");
      setOpen(false);
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const testDraft = useMutation({
    mutationFn: () => {
      const { config, credentials } = toPayload(form);
      return secretManagersApi.testUnsaved({ provider: form.provider, config, credentials });
    },
    onSuccess: setTestResult,
    onError: (e: Error) => setTestResult({ ok: false, message: e.message }),
  });

  const testSaved = useMutation({
    mutationFn: (id: string) => secretManagersApi.test(id),
    onSuccess: (r) => {
      invalidate();
      if (r.ok) toast.success(r.message);
      else toast.error(r.message);
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const remove = useMutation({
    mutationFn: (id: string) => secretManagersApi.delete(id),
    onSuccess: () => {
      invalidate();
      toast.success("Connection deleted");
      setDeleteTarget(null);
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const authMethods = AUTH_METHODS[form.provider];
  const auth = form.values.auth_method ?? authMethods?.[0]?.value;
  const visibleFields = FIELDS[form.provider].filter((f) => !f.auth || f.auth === auth);
  const setValue = (k: string, v: string) =>
    setForm((f) => ({ ...f, values: { ...f.values, [k]: v } }));

  return (
    <div className="container mx-auto py-6 space-y-6">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Secret managers</h1>
          <p className="text-muted-foreground text-sm mt-1">
            Reference secrets from HashiCorp Vault, OpenBao, Infisical, Doppler or AWS Secrets
            Manager in environment variables. Values are fetched at deploy time and never stored
            in Rivetr.
          </p>
        </div>
        <Button onClick={openCreate}>
          <Plus className="mr-2 h-4 w-4" />
          Add connection
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Connections</CardTitle>
          <CardDescription>
            Use a connection in an environment variable value as{" "}
            <code className="text-xs">secret://&lt;name&gt;/&lt;path&gt;#&lt;key&gt;</code>, or
            import every key at a path from the environment variables page.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {error ? (
            <QueryError error={error} onRetry={() => refetch()} />
          ) : isLoading ? (
            <div className="flex justify-center py-8">
              <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
            </div>
          ) : connections.length === 0 ? (
            <div className="text-center py-10 text-muted-foreground text-sm">
              <KeyRound className="mx-auto mb-2 h-8 w-8 opacity-50" />
              No secret manager connections yet.
            </div>
          ) : (
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Name</TableHead>
                    <TableHead>Provider</TableHead>
                    <TableHead className="hidden md:table-cell">Reference</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead className="w-[150px] text-right">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {connections.map((c) => (
                    <TableRow key={c.id}>
                      <TableCell className="font-mono text-sm">
                        {c.name}
                        {c.team_id === null && (
                          <Badge variant="outline" className="ml-2">Instance</Badge>
                        )}
                      </TableCell>
                      <TableCell>{PROVIDER_LABELS[c.provider] ?? c.provider}</TableCell>
                      <TableCell className="hidden md:table-cell font-mono text-xs text-muted-foreground">
                        {PATH_HINT[c.provider]?.replace("NAME", c.name)}
                      </TableCell>
                      <TableCell>
                        <TestBadge c={c} />
                      </TableCell>
                      <TableCell className="text-right whitespace-nowrap">
                        <Button
                          variant="ghost"
                          size="sm"
                          title="Test connection"
                          disabled={testSaved.isPending}
                          onClick={() => testSaved.mutate(c.id)}
                        >
                          {testSaved.isPending && testSaved.variables === c.id ? (
                            <Loader2 className="h-4 w-4 animate-spin" />
                          ) : (
                            <PlugZap className="h-4 w-4" />
                          )}
                        </Button>
                        <Button variant="ghost" size="sm" title="Edit" onClick={() => openEdit(c)}>
                          <Pencil className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          title="Delete"
                          onClick={() => setDeleteTarget(c)}
                        >
                          <Trash2 className="h-4 w-4 text-destructive" />
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="sm:max-w-lg max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>{editing ? `Edit ${editing.name}` : "Add secret manager"}</DialogTitle>
            <DialogDescription>
              Credentials are encrypted at rest and never shown again. Leave a credential blank
              when editing to keep the stored one.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="sm-name">Name</Label>
                <Input
                  id="sm-name"
                  value={form.name}
                  placeholder="vault-prod"
                  onChange={(e) =>
                    setForm((f) => ({ ...f, name: e.target.value.toLowerCase() }))
                  }
                />
              </div>
              <div className="space-y-2">
                <Label>Provider</Label>
                <Select
                  value={form.provider}
                  disabled={!!editing}
                  onValueChange={(v) =>
                    setForm((f) => ({ ...f, provider: v as SecretProviderKind, values: {} }))
                  }
                >
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {(Object.keys(PROVIDER_LABELS) as SecretProviderKind[]).map((p) => (
                      <SelectItem key={p} value={p}>
                        {PROVIDER_LABELS[p]}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
            {authMethods && (
              <div className="space-y-2">
                <Label>Authentication</Label>
                <Select value={auth} onValueChange={(v) => setValue("auth_method", v)}>
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {authMethods.map((m) => (
                      <SelectItem key={m.value} value={m.value}>
                        {m.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            )}
            {visibleFields.map((f) => {
              const stored = editing?.credentials_set.includes(f.key) ?? false;
              return (
                <div key={f.key} className="space-y-2">
                  <Label htmlFor={`sm-${f.key}`}>
                    {f.label}
                    {f.required && !stored && <span className="text-destructive"> *</span>}
                  </Label>
                  <Input
                    id={`sm-${f.key}`}
                    type={f.secret ? "password" : "text"}
                    autoComplete="off"
                    value={form.values[f.key] ?? ""}
                    placeholder={stored ? "Stored, leave blank to keep" : f.placeholder}
                    onChange={(e) => setValue(f.key, e.target.value)}
                  />
                  {f.help && <p className="text-xs text-muted-foreground">{f.help}</p>}
                </div>
              );
            })}
            {testResult && (
              <p
                className={`text-sm ${testResult.ok ? "text-green-600" : "text-destructive"}`}
                role="status"
              >
                {testResult.message}
              </p>
            )}
          </div>
          <DialogFooter className="gap-2">
            {!editing && (
              <Button
                variant="outline"
                onClick={() => testDraft.mutate()}
                disabled={testDraft.isPending}
              >
                {testDraft.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                Test connection
              </Button>
            )}
            <Button onClick={() => save.mutate()} disabled={save.isPending || !form.name}>
              {save.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {editing ? "Save" : "Add connection"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <AlertDialog open={!!deleteTarget} onOpenChange={(o) => !o && setDeleteTarget(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete {deleteTarget?.name}?</AlertDialogTitle>
            <AlertDialogDescription>
              Path imports using this connection are removed. Variables that reference{" "}
              <code>secret://{deleteTarget?.name}/...</code> will fail the next deploy until
              you point them elsewhere.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => deleteTarget && remove.mutate(deleteTarget.id)}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
