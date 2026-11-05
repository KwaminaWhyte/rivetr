import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { CheckCircle2, Loader2, Plus, Trash2 } from "lucide-react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { QueryError } from "@/components/query-error";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  DNS_PROVIDER_LABELS,
  dnsProvidersApi,
  type DnsCredentials,
  type DnsProviderKind,
} from "@/lib/api/certificates";
import { useIsInstanceAdmin } from "@/lib/auth";

export function meta() {
  return [
    { title: "DNS providers - Integrations - Rivetr" },
    {
      name: "description",
      content: "DNS provider credentials for wildcard certificates (ACME DNS-01)",
    },
  ];
}

const errorText = (e: unknown) => (e instanceof Error ? e.message : "Request failed");

function CredentialFields({
  provider,
  creds,
  onChange,
}: {
  provider: DnsProviderKind;
  creds: DnsCredentials;
  onChange: (c: DnsCredentials) => void;
}) {
  const field = (key: keyof DnsCredentials, label: string, opts?: { secret?: boolean; placeholder?: string }) => (
    <div className="space-y-2" key={key}>
      <Label htmlFor={`dns-${key}`}>{label}</Label>
      <Input
        id={`dns-${key}`}
        type={opts?.secret ? "password" : "text"}
        autoComplete="off"
        placeholder={opts?.placeholder}
        value={creds[key] ?? ""}
        onChange={(e) => onChange({ ...creds, [key]: e.target.value })}
      />
    </div>
  );
  if (provider === "route53") {
    return (
      <>
        {field("access_key_id", "Access key ID")}
        {field("secret_access_key", "Secret access key", { secret: true })}
        {field("region", "Region (optional)", { placeholder: "us-east-1" })}
        {field("hosted_zone_id", "Hosted zone ID (optional)", { placeholder: "Found automatically" })}
        <p className="text-xs text-muted-foreground">
          The key needs route53:ListHostedZones, route53:ListHostedZonesByName and
          route53:ChangeResourceRecordSets.
        </p>
      </>
    );
  }
  const hint: Record<string, string> = {
    cloudflare: "A Cloudflare API token with Zone:Read and DNS:Edit for the zones.",
    hetzner: "A Hetzner Cloud API token (read and write) for the project that holds the zones.",
    digitalocean: "A DigitalOcean personal access token with domain read and write access.",
  };
  return (
    <>
      {field("api_token", "API token", { secret: true })}
      <p className="text-xs text-muted-foreground">{hint[provider]}</p>
    </>
  );
}

function WildcardCard({ isAdmin }: { isAdmin: boolean }) {
  const qc = useQueryClient();
  const { data: providers = [] } = useQuery({
    queryKey: ["dns-providers"],
    queryFn: () => dnsProvidersApi.list(),
  });
  const { data: wildcard } = useQuery({
    queryKey: ["wildcard-certificate"],
    queryFn: () => dnsProvidersApi.getWildcard(),
    enabled: isAdmin,
  });
  const [domain, setDomain] = useState("");
  const [providerId, setProviderId] = useState<string>("");

  useEffect(() => {
    setDomain(wildcard?.domain ?? "");
    setProviderId(wildcard?.dns_provider_id ?? "");
  }, [wildcard?.domain, wildcard?.dns_provider_id]);

  const save = useMutation({
    mutationFn: () => dnsProvidersApi.setWildcard(domain, providerId || null),
    onSuccess: () => {
      toast.success(domain ? "Wildcard certificate requested" : "Wildcard certificate turned off");
      qc.invalidateQueries({ queryKey: ["wildcard-certificate"] });
      qc.invalidateQueries({ queryKey: ["dns-providers"] });
    },
    onError: (e) => toast.error(errorText(e)),
  });

  if (!isAdmin) return null;

  return (
    <Card>
      <CardHeader>
        <CardTitle>Wildcard instance domain</CardTitle>
        <CardDescription>
          One wildcard certificate for auto-generated and preview domains, such as{" "}
          <code className="text-xs">*.apps.example.com</code>. Domains it covers stop getting
          their own certificates.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="space-y-2">
            <Label htmlFor="wildcard-domain">Wildcard domain</Label>
            <Input
              id="wildcard-domain"
              placeholder={wildcard?.suggested_domain ?? "*.apps.example.com"}
              value={domain}
              onChange={(e) => setDomain(e.target.value)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="wildcard-provider">DNS provider</Label>
            <Select value={providerId} onValueChange={setProviderId}>
              <SelectTrigger id="wildcard-provider">
                <SelectValue placeholder={providers.length ? "Choose a provider" : "Add a provider first"} />
              </SelectTrigger>
              <SelectContent>
                {providers.map((p) => (
                  <SelectItem key={p.id} value={p.id}>
                    {p.name} ({DNS_PROVIDER_LABELS[p.provider]})
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>
        {wildcard?.domain && (
          <p className="text-sm text-muted-foreground">
            Status: {wildcard.status ?? "pending"}
            {wildcard.not_after && `, valid until ${new Date(wildcard.not_after).toLocaleDateString()}`}
            {wildcard.last_error && (
              <span className="block text-destructive break-words">{wildcard.last_error}</span>
            )}
          </p>
        )}
        <div className="flex flex-wrap gap-2">
          <Button
            size="sm"
            onClick={() => save.mutate()}
            disabled={save.isPending || (!!domain.trim() && !providerId)}
          >
            {save.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            Save
          </Button>
          {wildcard?.suggested_domain && !domain && (
            <Button size="sm" variant="outline" onClick={() => setDomain(wildcard.suggested_domain ?? "")}>
              Use {wildcard.suggested_domain}
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

export default function DnsProvidersPage() {
  const qc = useQueryClient();
  const isAdmin = useIsInstanceAdmin();
  const { data: providers = [], isLoading, error, refetch } = useQuery({
    queryKey: ["dns-providers"],
    queryFn: () => dnsProvidersApi.list(),
  });

  const [addOpen, setAddOpen] = useState(false);
  const [editId, setEditId] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [kind, setKind] = useState<DnsProviderKind>("cloudflare");
  const [creds, setCreds] = useState<DnsCredentials>({});
  const [deleteId, setDeleteId] = useState<string | null>(null);

  const editing = providers.find((p) => p.id === editId);

  const reset = () => {
    setAddOpen(false);
    setEditId(null);
    setName("");
    setKind("cloudflare");
    setCreds({});
  };

  const save = useMutation({
    mutationFn: () =>
      editing
        ? dnsProvidersApi.update(editing.id, {
            name,
            credentials: Object.values(creds).some((v) => v) ? creds : undefined,
          })
        : dnsProvidersApi.create({ name, provider: kind, credentials: creds }),
    onSuccess: () => {
      toast.success(editing ? "DNS provider updated" : "DNS provider added");
      qc.invalidateQueries({ queryKey: ["dns-providers"] });
      reset();
    },
    onError: (e) => toast.error(errorText(e)),
  });

  const remove = useMutation({
    mutationFn: (id: string) => dnsProvidersApi.delete(id),
    onSuccess: () => {
      toast.success("DNS provider deleted");
      qc.invalidateQueries({ queryKey: ["dns-providers"] });
      setDeleteId(null);
    },
    onError: (e) => {
      toast.error(errorText(e));
      setDeleteId(null);
    },
  });

  const verify = useMutation({
    mutationFn: (id: string) => dnsProvidersApi.verify(id),
    onSuccess: (r) => (r.ok ? toast.success(r.message) : toast.error(r.message)),
    onError: (e) => toast.error(errorText(e)),
  });

  const openEdit = (id: string) => {
    const p = providers.find((x) => x.id === id);
    if (!p) return;
    setEditId(id);
    setName(p.name);
    setKind(p.provider);
    setCreds({});
  };

  return (
    <div className="container mx-auto py-6 space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">DNS providers</h1>
          <p className="text-muted-foreground text-sm mt-1">
            Credentials Rivetr uses to create <code className="text-xs">_acme-challenge</code> TXT
            records, so Let's Encrypt can issue wildcard certificates (DNS-01).
          </p>
        </div>
        {isAdmin && (
          <Button onClick={() => setAddOpen(true)}>
            <Plus className="mr-2 h-4 w-4" />
            Add provider
          </Button>
        )}
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Providers</CardTitle>
          <CardDescription>
            Pick a provider per domain under the app's Domains page, Certificates, "Wildcard via DNS
            provider". Credentials are encrypted at rest.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {error ? (
            <QueryError error={error} onRetry={() => refetch()} />
          ) : isLoading ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : providers.length === 0 ? (
            <p className="text-sm text-muted-foreground">No DNS providers yet.</p>
          ) : (
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Name</TableHead>
                    <TableHead>Provider</TableHead>
                    <TableHead>Used by</TableHead>
                    {isAdmin && <TableHead className="text-right">Actions</TableHead>}
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {providers.map((p) => (
                    <TableRow key={p.id}>
                      <TableCell>
                        <div className="font-medium">{p.name}</div>
                        <div className="text-xs text-muted-foreground">{p.summary}</div>
                      </TableCell>
                      <TableCell>
                        <Badge variant="outline">{DNS_PROVIDER_LABELS[p.provider] ?? p.provider}</Badge>
                      </TableCell>
                      <TableCell className="text-xs">
                        {p.used_by.length ? p.used_by.join(", ") : "Not used"}
                      </TableCell>
                      {isAdmin && (
                        <TableCell className="text-right whitespace-nowrap">
                          <Button
                            variant="ghost"
                            size="sm"
                            disabled={verify.isPending}
                            onClick={() => verify.mutate(p.id)}
                          >
                            <CheckCircle2 className="mr-1 h-4 w-4" />
                            Test
                          </Button>
                          <Button variant="ghost" size="sm" onClick={() => openEdit(p.id)}>
                            Edit
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            aria-label={`Delete ${p.name}`}
                            onClick={() => setDeleteId(p.id)}
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        </TableCell>
                      )}
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>

      <WildcardCard isAdmin={isAdmin} />

      <Dialog open={addOpen || editId !== null} onOpenChange={(o) => !o && reset()}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>{editing ? "Edit DNS provider" : "Add DNS provider"}</DialogTitle>
            <DialogDescription>
              {editing
                ? "Leave the credentials empty to keep the stored ones."
                : "Rivetr only creates and removes TXT records under _acme-challenge."}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="dns-name">Name</Label>
              <Input
                id="dns-name"
                placeholder="Production Cloudflare"
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="dns-kind">Provider</Label>
              <Select
                value={kind}
                onValueChange={(v) => {
                  setKind(v as DnsProviderKind);
                  setCreds({});
                }}
                disabled={!!editing}
              >
                <SelectTrigger id="dns-kind">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {(Object.keys(DNS_PROVIDER_LABELS) as DnsProviderKind[]).map((k) => (
                    <SelectItem key={k} value={k}>
                      {DNS_PROVIDER_LABELS[k]}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <CredentialFields provider={kind} creds={creds} onChange={setCreds} />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={reset}>
              Cancel
            </Button>
            <Button disabled={!name.trim() || save.isPending} onClick={() => save.mutate()}>
              {save.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {editing ? "Save" : "Add"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <AlertDialog open={deleteId !== null} onOpenChange={(o) => !o && setDeleteId(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete DNS provider?</AlertDialogTitle>
            <AlertDialogDescription>
              Certificates already issued keep working until they expire. A provider that is still
              in use cannot be deleted.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={() => deleteId && remove.mutate(deleteId)}>
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
