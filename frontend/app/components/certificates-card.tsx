import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Loader2, ShieldCheck, Trash2, Upload } from "lucide-react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
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
  certificatesApi,
  DNS_PROVIDER_LABELS,
  dnsProvidersApi,
  type CertificateResource,
  type DomainCertificate,
} from "@/lib/api/certificates";

function statusBadge(cert: DomainCertificate) {
  switch (cert.status) {
    case "issued":
      return <Badge className="bg-green-600 text-white">Issued</Badge>;
    case "custom":
      return <Badge className="bg-green-600 text-white">Custom</Badge>;
    case "pending":
      return <Badge variant="secondary">Pending</Badge>;
    case "expiring":
      return <Badge className="bg-amber-500 text-white">Expires soon</Badge>;
    case "expired":
      return <Badge variant="destructive">Expired</Badge>;
    case "failed":
      return <Badge variant="destructive">Failed</Badge>;
    default:
      return <Badge variant="outline">HTTPS off</Badge>;
  }
}

function formatDate(iso: string | null) {
  if (!iso) return null;
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleDateString();
}

interface Props {
  resource: CertificateResource;
  resourceId: string;
  /** Changes when the resource's domains change, to refetch the list. */
  version?: string;
}

/** Certificate status and settings for each domain of an app or service. */
export function CertificatesCard({ resource, resourceId, version }: Props) {
  const qc = useQueryClient();
  const queryKey = ["certificates", resource, resourceId, version ?? ""];
  const { data: certs, isLoading } = useQuery({
    queryKey,
    queryFn: () => certificatesApi.list(resource, resourceId),
    refetchInterval: (q) =>
      q.state.data?.some((c) => c.status === "pending") ? 15_000 : false,
  });
  const { data: providers = [] } = useQuery({
    queryKey: ["dns-providers"],
    queryFn: () => dnsProvidersApi.list(),
  });
  const [uploadFor, setUploadFor] = useState<string | null>(null);
  const [certPem, setCertPem] = useState("");
  const [keyPem, setKeyPem] = useState("");

  const refresh = () =>
    qc.invalidateQueries({ queryKey: ["certificates", resource, resourceId] });
  const onError = (e: unknown) =>
    toast.error(e instanceof Error ? e.message : "Request failed");

  const upload = useMutation({
    mutationFn: () =>
      certificatesApi.upload(resource, resourceId, uploadFor ?? "", certPem, keyPem),
    onSuccess: () => {
      toast.success("Certificate uploaded");
      setUploadFor(null);
      setCertPem("");
      setKeyPem("");
      refresh();
    },
    onError,
  });

  const removeCustom = useMutation({
    mutationFn: (domain: string) =>
      certificatesApi.removeCustom(resource, resourceId, domain),
    onSuccess: () => {
      toast.success("Custom certificate removed; automatic issuance resumes");
      refresh();
    },
    onError,
  });

  const setMode = useMutation({
    mutationFn: ({ domain, providerId }: { domain: string; providerId: string | null }) =>
      certificatesApi.setMode(
        resource,
        resourceId,
        domain,
        providerId ? "dns" : "auto",
        providerId ?? undefined,
      ),
    onSuccess: () => {
      toast.success("Certificate setting saved; issuance starts shortly");
      refresh();
    },
    onError,
  });

  /** Select values: "auto", "custom" or "dns:<provider id>". */
  const modeValue = (cert: DomainCertificate) =>
    cert.mode === "dns" && cert.dns_provider_id ? `dns:${cert.dns_provider_id}` : cert.mode;

  const changeMode = (cert: DomainCertificate, value: string) => {
    if (value === "custom") {
      setUploadFor(cert.domain);
      return;
    }
    const providerId = value.startsWith("dns:") ? value.slice(4) : null;
    setMode.mutate({ domain: cert.domain, providerId });
  };

  const readFile = async (file: File | undefined, set: (v: string) => void) => {
    if (file) set(await file.text());
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <ShieldCheck className="h-5 w-5" />
          Certificates
        </CardTitle>
        <CardDescription>
          Each domain gets its own TLS certificate. Choose automatic Let's Encrypt issuance, a
          wildcard for the domain and its subdomains through a DNS provider (set up under
          Integrations, DNS providers), or upload your own certificate.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        {isLoading && <Loader2 className="h-4 w-4 animate-spin" />}
        {certs && certs.length === 0 && (
          <p className="text-sm text-muted-foreground">Add a domain to manage its certificate.</p>
        )}
        {certs && certs.length > 0 && !certs[0].https_enabled && (
          <p className="text-sm text-muted-foreground">
            HTTPS is off on this server. Enable ACME in the proxy configuration, or upload a
            certificate and restart Rivetr to start the HTTPS listener.
          </p>
        )}
        {certs?.map((cert) => (
          <div
            key={cert.domain}
            className="flex flex-col gap-3 rounded-md border p-3 sm:flex-row sm:items-start sm:justify-between"
          >
            <div className="min-w-0 space-y-1">
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-mono text-sm break-all">{cert.domain}</span>
                {statusBadge(cert)}
              </div>
              <p className="text-xs text-muted-foreground">
                {cert.not_after
                  ? `${cert.issuer || "Unknown issuer"}, valid until ${formatDate(cert.not_after)}`
                  : cert.acme_enabled
                    ? "No certificate yet"
                    : "No certificate"}
                {cert.serving === "legacy" && " (shared certificate, being replaced)"}
              </p>
              {cert.last_error && cert.status === "failed" && (
                <p className="text-xs text-destructive break-words">
                  {cert.last_error}
                  {cert.next_attempt_at && ` Next try: ${new Date(cert.next_attempt_at).toLocaleString()}.`}
                </p>
              )}
            </div>
            <div className="flex shrink-0 items-center gap-2">
              <Select
                value={modeValue(cert)}
                onValueChange={(v) => changeMode(cert, v)}
                disabled={setMode.isPending}
              >
                <SelectTrigger aria-label={`Certificate for ${cert.domain}`} className="w-64 max-w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="auto">Automatic (HTTP-01)</SelectItem>
                  {providers.map((p) => (
                    <SelectItem key={p.id} value={`dns:${p.id}`}>
                      Wildcard via {p.name} ({DNS_PROVIDER_LABELS[p.provider] ?? p.provider})
                    </SelectItem>
                  ))}
                  <SelectItem value="custom">Custom upload</SelectItem>
                </SelectContent>
              </Select>
              {cert.mode === "custom" && (
                <>
                  <Button
                    variant="outline"
                    size="icon"
                    aria-label="Replace certificate"
                    onClick={() => setUploadFor(cert.domain)}
                  >
                    <Upload className="h-4 w-4" />
                  </Button>
                  <Button
                    variant="outline"
                    size="icon"
                    aria-label="Remove custom certificate"
                    disabled={removeCustom.isPending}
                    onClick={() => removeCustom.mutate(cert.domain)}
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </>
              )}
            </div>
          </div>
        ))}
      </CardContent>

      <Dialog open={uploadFor !== null} onOpenChange={(o) => !o && setUploadFor(null)}>
        <DialogContent className="sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle>Upload certificate</DialogTitle>
            <DialogDescription>
              For <span className="font-mono">{uploadFor}</span>. Paste the PEM certificate chain
              (leaf first) and its private key. The key is encrypted at rest and the certificate
              takes priority over automatic issuance.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label htmlFor="cert-pem">Certificate chain</Label>
                <input
                  type="file"
                  accept=".pem,.crt,.cer"
                  aria-label="Load certificate file"
                  className="text-xs"
                  onChange={(e) => readFile(e.target.files?.[0], setCertPem)}
                />
              </div>
              <Textarea
                id="cert-pem"
                rows={7}
                className="font-mono text-xs"
                placeholder="-----BEGIN CERTIFICATE-----"
                value={certPem}
                onChange={(e) => setCertPem(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label htmlFor="key-pem">Private key</Label>
                <input
                  type="file"
                  accept=".pem,.key"
                  aria-label="Load private key file"
                  className="text-xs"
                  onChange={(e) => readFile(e.target.files?.[0], setKeyPem)}
                />
              </div>
              <Textarea
                id="key-pem"
                rows={5}
                className="font-mono text-xs"
                placeholder="-----BEGIN PRIVATE KEY-----"
                value={keyPem}
                onChange={(e) => setKeyPem(e.target.value)}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setUploadFor(null)}>
              Cancel
            </Button>
            <Button
              disabled={!certPem.trim() || !keyPem.trim() || upload.isPending}
              onClick={() => upload.mutate()}
            >
              {upload.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              Upload
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Card>
  );
}
