/**
 * Secret manager card for an env var page: imported paths and, for apps and
 * services, what happens when a provider is unreachable during a deploy.
 */
import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { FolderInput, Lock, Trash2 } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  PROVIDER_LABELS,
  secretManagersApi,
  type SecretFailurePolicy,
  type SecretOwnerType,
} from "@/lib/api/secret-managers";
import { SecretPickerDialog } from "./secret-picker";

export function SecretManagerSection({
  ownerType,
  ownerId,
}: {
  ownerType: SecretOwnerType;
  ownerId: string;
}) {
  const qc = useQueryClient();
  const [importOpen, setImportOpen] = useState(false);
  const hasPolicy = ownerType !== "project";

  const { data: imports = [] } = useQuery({
    queryKey: ["secret-imports", ownerType, ownerId],
    queryFn: () => secretManagersApi.listImports(ownerType, ownerId),
  });

  const { data: policy } = useQuery({
    queryKey: ["secret-policy", ownerType, ownerId],
    queryFn: () => secretManagersApi.getPolicy(ownerType as "app" | "service", ownerId),
    enabled: hasPolicy,
  });

  const removeImport = useMutation({
    mutationFn: (id: string) => secretManagersApi.deleteImport(ownerType, ownerId, id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["secret-imports", ownerType, ownerId] });
      toast.success("Import removed");
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const setPolicy = useMutation({
    mutationFn: (p: SecretFailurePolicy) =>
      secretManagersApi.setPolicy(ownerType as "app" | "service", ownerId, p),
    onSuccess: (data) => {
      qc.setQueryData(["secret-policy", ownerType, ownerId], data);
      toast.success("Saved");
    },
    onError: (e: Error) => toast.error(e.message),
  });

  return (
    <Card>
      <CardHeader>
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="space-y-1.5">
            <CardTitle className="flex items-center gap-2">
              <Lock className="h-5 w-5" />
              Secret manager
            </CardTitle>
            <CardDescription>
              Values like <code>secret://vault-prod/app/config#DATABASE_URL</code> are fetched at
              deploy time, masked in deployment logs and never stored in Rivetr.
              {ownerType === "project" && " Imports here apply to every app in the project."}
            </CardDescription>
          </div>
          <Button variant="outline" size="sm" className="gap-2" onClick={() => setImportOpen(true)}>
            <FolderInput className="h-4 w-4" />
            Import path
          </Button>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {imports.length === 0 ? (
          <p className="text-sm text-muted-foreground">No imported paths.</p>
        ) : (
          <ul className="divide-y rounded-md border">
            {imports.map((i) => (
              <li key={i.id} className="flex items-center justify-between gap-2 p-2 text-sm">
                <div className="min-w-0">
                  <span className="font-mono break-all">
                    {i.connection_name ?? "(deleted connection)"}/{i.path}
                  </span>
                  <span className="ml-2 text-xs text-muted-foreground">
                    {i.provider ? PROVIDER_LABELS[i.provider] : ""}
                    {i.prefix ? `, prefix ${i.prefix}` : ""}
                  </span>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  aria-label="Remove import"
                  onClick={() => removeImport.mutate(i.id)}
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </li>
            ))}
          </ul>
        )}

        {hasPolicy && (
          <div className="space-y-2">
            <Label>If a secret manager is unreachable during a deploy</Label>
            <Select
              value={policy?.failure_policy ?? "fail"}
              onValueChange={(v) => setPolicy.mutate(v as SecretFailurePolicy)}
            >
              <SelectTrigger className="w-full sm:w-96">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="fail">Fail the deploy (default)</SelectItem>
                <SelectItem value="last_known">
                  Use the last resolved values, with a warning
                </SelectItem>
              </SelectContent>
            </Select>
            {policy?.failure_policy === "last_known" && (
              <p className="text-xs text-muted-foreground">
                The last resolved values are kept encrypted
                {policy.last_resolved_at
                  ? ` (last stored ${new Date(policy.last_resolved_at).toLocaleString()})`
                  : " after the next successful deploy"}
                .
              </p>
            )}
          </div>
        )}
      </CardContent>
      <SecretPickerDialog
        open={importOpen}
        onOpenChange={setImportOpen}
        mode="import"
        ownerType={ownerType}
        ownerId={ownerId}
      />
    </Card>
  );
}
