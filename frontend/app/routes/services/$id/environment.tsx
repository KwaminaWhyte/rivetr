import { useState } from "react";
import { useOutletContext } from "react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { api } from "@/lib/api";
import type { Service, ServiceDeployConfig, ServiceEnvVar } from "@/types/api";
import { ServiceSettingsSections } from "@/components/services/service-settings-sections";
import {
  FromSecretManagerButton,
  SecretRefChip,
  isSecretRefValue,
} from "@/components/secret-managers/secret-picker";
import { SecretManagerSection } from "@/components/secret-managers/secret-manager-section";
import { Eye, EyeOff, FileUp, KeyRound, Pencil, Plus, Trash2 } from "lucide-react";

export function meta() {
  return [
    { title: "Service Environment - Rivetr" },
    { name: "description", content: "Environment variables and env file for this service" },
  ];
}

interface OutletContext {
  service: Service;
}

function EditDialog({
  service,
  existing,
  onClose,
}: {
  service: Service;
  existing: ServiceEnvVar | null;
  onClose: () => void;
}) {
  const queryClient = useQueryClient();
  const [key, setKey] = useState(existing?.key ?? "");
  const masked = existing?.value === "********";
  const initialValue = existing && !masked ? existing.value : "";
  const [value, setValue] = useState(initialValue);
  const [isSecret, setIsSecret] = useState(existing?.is_secret ?? true);
  const mutation = useMutation({
    mutationFn: () =>
      existing
        ? api.updateServiceEnvVar(service.id, existing.key, {
            ...(value !== "" || !masked ? { value } : {}),
            is_secret: isSecret,
          })
        : api.createServiceEnvVar(service.id, { key, value, is_secret: isSecret }),
    onSuccess: () => {
      toast.success(existing ? "Variable updated" : "Variable added");
      queryClient.invalidateQueries({ queryKey: ["service-env", service.id] });
      onClose();
    },
    onError: (e) => toast.error(e instanceof Error ? e.message : "Failed to save"),
  });
  return (
    <Dialog open onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{existing ? `Edit ${existing.key}` : "Add variable"}</DialogTitle>
        </DialogHeader>
        <div className="space-y-3">
          {!existing && (
            <div className="grid gap-1.5">
              <Label htmlFor="env-key">Name</Label>
              <Input
                id="env-key"
                value={key}
                onChange={(e) => setKey(e.target.value)}
                placeholder="DATABASE_URL"
                className="font-mono"
              />
            </div>
          )}
          <div className="grid gap-1.5">
            <Label htmlFor="env-value">Value</Label>
            <Textarea
              id="env-value"
              rows={3}
              className="font-mono text-xs"
              value={value}
              onChange={(e) => setValue(e.target.value)}
              placeholder={masked ? "Leave empty to keep the stored value" : ""}
            />
          </div>
          <div className="flex items-center gap-3">
            <Switch id="env-secret" checked={isSecret} onCheckedChange={setIsSecret} />
            <Label htmlFor="env-secret">Secret (hidden in the dashboard)</Label>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={() => mutation.mutate()} disabled={mutation.isPending}>
            Save
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function ImportDialog({ service, onClose }: { service: Service; onClose: () => void }) {
  const queryClient = useQueryClient();
  const [content, setContent] = useState("");
  const [overwrite, setOverwrite] = useState(false);
  const [isSecret, setIsSecret] = useState(true);
  const mutation = useMutation({
    mutationFn: () =>
      api.importServiceEnvVars(service.id, { content, overwrite, is_secret: isSecret }),
    onSuccess: (r) => {
      toast.success(
        `Imported: ${r.created} added, ${r.updated} updated` +
          (r.skipped.length ? `, ${r.skipped.length} skipped (${r.skipped.join(", ")})` : ""),
      );
      queryClient.invalidateQueries({ queryKey: ["service-env", service.id] });
      onClose();
    },
    onError: (e) => toast.error(e instanceof Error ? e.message : "Import failed"),
  });
  return (
    <Dialog open onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Import a .env file</DialogTitle>
          <DialogDescription>
            Paste the contents of an env file (for example the one on the server that the compose
            file references today). Every KEY=VALUE line becomes a variable.
          </DialogDescription>
        </DialogHeader>
        <Textarea
          rows={12}
          className="font-mono text-xs"
          value={content}
          onChange={(e) => setContent(e.target.value)}
          placeholder={"DATABASE_URL=postgres://...\nSESSION_SECRET=..."}
        />
        <div className="flex flex-wrap gap-6">
          <div className="flex items-center gap-3">
            <Switch id="imp-secret" checked={isSecret} onCheckedChange={setIsSecret} />
            <Label htmlFor="imp-secret">Mark as secret</Label>
          </div>
          <div className="flex items-center gap-3">
            <Switch id="imp-over" checked={overwrite} onCheckedChange={setOverwrite} />
            <Label htmlFor="imp-over">Replace existing values</Label>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button
            onClick={() => mutation.mutate()}
            disabled={!content.trim() || mutation.isPending}
          >
            Import
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export default function ServiceEnvironmentTab() {
  const { service } = useOutletContext<OutletContext>();
  const queryClient = useQueryClient();
  const [reveal, setReveal] = useState(false);
  const [editing, setEditing] = useState<ServiceEnvVar | "new" | null>(null);
  const [importing, setImporting] = useState(false);

  const { data: vars = [], isLoading } = useQuery<ServiceEnvVar[]>({
    queryKey: ["service-env", service.id, reveal],
    queryFn: () => api.getServiceEnvVars(service.id, reveal),
  });
  const { data: config } = useQuery<ServiceDeployConfig>({
    queryKey: ["service-deploy-config", service.id],
    queryFn: () => api.getServiceDeployConfig(service.id),
  });

  const injectMutation = useMutation({
    mutationFn: (on: boolean) => api.updateServiceDeployConfig(service.id, { inject_env_file: on }),
    onSuccess: (data) => {
      queryClient.setQueryData(["service-deploy-config", service.id], data);
      toast.success("Saved. Deploy or restart to apply.");
    },
    onError: (e) => toast.error(e instanceof Error ? e.message : "Failed to save"),
  });

  const deleteMutation = useMutation({
    mutationFn: (key: string) => api.deleteServiceEnvVar(service.id, key),
    onSuccess: () => {
      toast.success("Variable deleted");
      queryClient.invalidateQueries({ queryKey: ["service-env", service.id] });
    },
    onError: (e) => toast.error(e instanceof Error ? e.message : "Failed to delete"),
  });

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="space-y-1.5">
              <CardTitle className="flex items-center gap-2">
                <KeyRound className="h-5 w-5" />
                Environment variables
              </CardTitle>
              <CardDescription>
                Stored encrypted. Rivetr writes them to a <code>.env</code> file next to the compose
                file (readable only by Rivetr) on every start and deploy. Changes apply on the next
                deploy or restart.
              </CardDescription>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button
                variant="outline"
                size="sm"
                className="gap-2"
                onClick={() => setReveal((r) => !r)}
              >
                {reveal ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                {reveal ? "Hide" : "Reveal"}
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="gap-2"
                onClick={() => setImporting(true)}
              >
                <FileUp className="h-4 w-4" />
                Import .env
              </Button>
              <FromSecretManagerButton
                ownerType="service"
                ownerId={service.id}
                createVariable={(key, value) =>
                  api.createServiceEnvVar(service.id, { key, value, is_secret: false })
                }
                invalidate={[["service-env", service.id]]}
              />
              <Button size="sm" className="gap-2" onClick={() => setEditing("new")}>
                <Plus className="h-4 w-4" />
                Add
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="h-20 animate-pulse rounded bg-muted" />
          ) : vars.length === 0 ? (
            <p className="py-6 text-center text-sm text-muted-foreground">No variables yet.</p>
          ) : (
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Name</TableHead>
                    <TableHead>Value</TableHead>
                    <TableHead className="text-right">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {vars.map((v) => (
                    <TableRow key={v.id}>
                      <TableCell className="font-mono text-sm">
                        {v.key}
                        {v.is_secret && (
                          <Badge variant="outline" className="ml-2 text-[10px]">
                            secret
                          </Badge>
                        )}
                      </TableCell>
                      <TableCell className="max-w-md break-all font-mono text-xs">
                        {isSecretRefValue(v.value) ? <SecretRefChip value={v.value} /> : v.value}
                      </TableCell>
                      <TableCell>
                        <div className="flex justify-end gap-1">
                          <Button
                            variant="ghost"
                            size="sm"
                            aria-label={`Edit ${v.key}`}
                            onClick={() => setEditing(v)}
                          >
                            <Pencil className="h-4 w-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            aria-label={`Delete ${v.key}`}
                            onClick={() => {
                              if (window.confirm(`Delete ${v.key}?`)) deleteMutation.mutate(v.key);
                            }}
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>

      <SecretManagerSection ownerType="service" ownerId={service.id} />

      <Card>
        <CardHeader>
          <CardTitle>Using the variables in the compose file</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4 text-sm">
          <div className="flex items-center gap-3">
            <Switch
              id="inject"
              checked={config?.inject_env_file ?? false}
              disabled={!config || injectMutation.isPending}
              onCheckedChange={(on) => injectMutation.mutate(on)}
            />
            <Label htmlFor="inject">Add the .env file to every container</Label>
          </div>
          <ul className="list-disc space-y-1 pl-5 text-muted-foreground">
            <li>
              With the switch on, Rivetr adds <code>env_file: .env</code> to each compose service,
              after any env_file it already lists.
            </li>
            <li>
              Or reference it only where needed:{" "}
              <code>
                env_file:
                <br />
                {"  "}- .env
              </code>
            </li>
            <li>
              Compose also reads it for <code>{"${VAR}"}</code> interpolation in the compose file.
            </li>
            <li>
              An existing <code>env_file</code> with an absolute path on the server (such as{" "}
              <code>/etc/myapp/app.env</code>) keeps working. To move it here, import that file
              above, switch it to <code>.env</code> in the compose file, and deploy.
            </li>
            <li>Pre- and post-deploy hooks get the same env as their compose service.</li>
          </ul>
        </CardContent>
      </Card>

      {editing && (
        <EditDialog
          service={service}
          existing={editing === "new" ? null : editing}
          onClose={() => setEditing(null)}
        />
      )}
      {importing && <ImportDialog service={service} onClose={() => setImporting(false)} />}
      <ServiceSettingsSections service={service} sections={["variables"]} />
    </div>
  );
}
