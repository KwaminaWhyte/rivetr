import { useState, useCallback, useMemo } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
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
import { Plus, Trash2, Globe, Star, ExternalLink, Copy, RefreshCw, CheckCircle2, AlertTriangle, XCircle, Wand2 } from "lucide-react";
import type { Domain, App, UpdateAppRequest } from "@/types/api";
import { apiRequest, appsApi } from "@/lib/api";
import { UnsavedChangesBar } from "@/components/unsaved-changes-bar";
import { useDirtyForm } from "@/hooks/use-dirty-form";

interface DnsCheckResult {
  domain: string;
  resolves: boolean;
  points_to_server: boolean;
  resolved_ips: string[];
  server_ip: string;
  /** DNS validation is turned off in Settings > Advanced */
  validation_disabled?: boolean;
}

function checkDns(domain: string): Promise<DnsCheckResult> {
  return apiRequest<DnsCheckResult>(`/domains/check?domain=${encodeURIComponent(domain)}`);
}

function DnsStatusBadge({ result, loading }: { result: DnsCheckResult | null | undefined; loading: boolean }) {
  if (loading) {
    return (
      <Badge variant="outline" className="gap-1 text-xs">
        <RefreshCw className="h-3 w-3 animate-spin" />
        Checking...
      </Badge>
    );
  }
  if (!result) {
    return (
      <Badge variant="outline" className="text-xs text-muted-foreground">
        Not checked
      </Badge>
    );
  }
  if (result.validation_disabled) return null;

  if (result.points_to_server) {
    return (
      <Badge variant="outline" className="gap-1 text-xs text-green-600 border-green-300">
        <CheckCircle2 className="h-3 w-3" />
        DNS OK
      </Badge>
    );
  }
  if (result.resolves) {
    return (
      <Badge
        variant="outline"
        className="gap-1 text-xs text-yellow-600 border-yellow-300"
        title={`Resolves to: ${result.resolved_ips.join(", ")}${result.server_ip ? `; this server is ${result.server_ip}` : ""}`}
      >
        <AlertTriangle className="h-3 w-3" />
        {result.server_ip ? "Wrong IP" : "Resolves"}
      </Badge>
    );
  }
  return (
    <Badge variant="outline" className="gap-1 text-xs text-red-600 border-red-300">
      <XCircle className="h-3 w-3" />
      Not resolving
    </Badge>
  );
}

interface DomainManagementCardProps {
  app: App;
  onSave: (updates: UpdateAppRequest) => Promise<void>;
  isSaving?: boolean;
}

// Helper to parse domains JSON from the app
function parseDomains(json: string | null): Domain[] {
  if (!json) return [];
  try {
    return JSON.parse(json);
  } catch {
    return [];
  }
}

export function DomainManagementCard({
  app,
  onSave,
  isSaving = false,
}: DomainManagementCardProps) {
  // Saved domains from the app; the form resets to them after a save.
  const initial = useMemo(() => ({ domains: parseDomains(app.domains) }), [app.domains]);
  const form = useDirtyForm(initial);
  const domains = form.values.domains;
  const setDomains = (next: Domain[]) => form.setField("domains", next);

  // Form state for adding new domain
  const [newDomain, setNewDomain] = useState({
    domain: "",
    primary: false,
    www_redirect_mode: "none" as "none" | "both" | "to_www" | "to_non_www",
  });

  // Generate domain state
  const [isGeneratingDomain, setIsGeneratingDomain] = useState(false);

  // DNS check state: map of domain -> result, null (failed) or "loading"
  const [dnsResults, setDnsResults] = useState<Record<string, DnsCheckResult | null | "loading">>({});

  const runDnsCheck = useCallback(async (domain: string) => {
    setDnsResults((prev) => ({ ...prev, [domain]: "loading" }));
    try {
      const result = await checkDns(domain);
      setDnsResults((prev) => ({ ...prev, [domain]: result }));
    } catch (error) {
      setDnsResults((prev) => ({ ...prev, [domain]: null }));
      toast.error(error instanceof Error ? error.message : "DNS check failed");
    }
  }, []);

  // Validate domain format
  const isValidDomain = (domain: string): boolean => {
    const pattern = /^[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?)*$/;
    return pattern.test(domain) && domain.length <= 253;
  };

  // Add new domain
  const addDomain = () => {
    const domainName = newDomain.domain.trim().toLowerCase();

    if (!domainName) {
      toast.error("Domain name is required");
      return;
    }

    if (!isValidDomain(domainName)) {
      toast.error("Invalid domain name format");
      return;
    }

    // Check for duplicates
    if (domains.some((d) => d.domain.toLowerCase() === domainName)) {
      toast.error("Domain already exists");
      return;
    }

    // If this is marked as primary, unmark others
    let updatedDomains = [...domains];
    if (newDomain.primary) {
      updatedDomains = updatedDomains.map((d) => ({ ...d, primary: false }));
    }

    const newEntry: Domain = {
      domain: domainName,
      primary: newDomain.primary || domains.length === 0, // First domain is always primary
      redirect_www: false,
      www_redirect_mode: newDomain.www_redirect_mode === "none" ? undefined : newDomain.www_redirect_mode,
      search_indexing: true,
    };

    setDomains([...updatedDomains, newEntry]);
    setNewDomain({ domain: "", primary: false, www_redirect_mode: "none" });
  };

  // Remove domain
  const removeDomain = (index: number) => {
    const removedDomain = domains[index];
    const newDomains = domains.filter((_, i) => i !== index);

    // If we removed the primary domain, make the first remaining domain primary
    if (removedDomain.primary && newDomains.length > 0) {
      newDomains[0] = { ...newDomains[0], primary: true };
    }

    setDomains(newDomains);
  };

  // Set domain as primary
  const setPrimaryDomain = (index: number) => {
    setDomains(domains.map((d, i) => ({ ...d, primary: i === index })));
  };

  const updateDomain = (index: number, patch: Partial<Domain>) => {
    setDomains(domains.map((d, i) => (i === index ? { ...d, ...patch } : d)));
  };

  // Copy domain to clipboard
  const copyDomain = (domain: string) => {
    navigator.clipboard.writeText(domain);
    toast.success("Domain copied to clipboard");
  };

  // Generate a random domain via the backend
  const handleGenerateDomain = async () => {
    setIsGeneratingDomain(true);
    try {
      const result = await appsApi.generateDomain(app.id);
      // Pre-fill the new domain input field (strip protocol if present)
      const domain = result.domain.replace(/^https?:\/\//, "");
      setNewDomain({ ...newDomain, domain });
      toast.success(`Generated domain: ${domain}`);
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to generate domain"
      );
    } finally {
      setIsGeneratingDomain(false);
    }
  };

  const invalidPort = domains.find(
    (d) => d.internal_port != null && (!Number.isInteger(d.internal_port) || d.internal_port < 1 || d.internal_port > 65535)
  );

  // Save handler
  const handleSave = async () => {
    if (invalidPort) {
      toast.error(`Internal port for ${invalidPort.domain} must be between 1 and 65535`);
      return;
    }
    try {
      await onSave({ domains });
      form.markSaved();
      toast.success("Domain configuration saved");
    } catch (error) {
      toast.error(
        `Failed to save: ${error instanceof Error ? error.message : "Unknown error"}`
      );
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Globe className="h-5 w-5" />
          Domain Management
        </CardTitle>
        <CardDescription>
          Configure custom domains for your application. All domains will route to
          your app. The primary domain will be used for redirects and canonical URLs.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Auto-generated subdomain display */}
        {app.auto_subdomain && (
          <div className="space-y-2 p-3 bg-muted/50 rounded-lg">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Badge variant="secondary">Auto-generated</Badge>
                <span className="font-mono text-sm">{app.auto_subdomain}</span>
              </div>
              <div className="flex gap-1">
                <Button aria-label="Copy to clipboard"
                  variant="ghost"
                  size="sm"
                  onClick={() => copyDomain(app.auto_subdomain!)}
                  className="h-7 w-7 p-0"
                >
                  <Copy className="h-4 w-4" />
                </Button>
                <Button aria-label="Open in new tab"
                  variant="ghost"
                  size="sm"
                  asChild
                  className="h-7 w-7 p-0"
                >
                  <a
                    href={`https://${app.auto_subdomain}`}
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    <ExternalLink className="h-4 w-4" />
                  </a>
                </Button>
              </div>
            </div>
            <p className="text-xs text-muted-foreground">
              This subdomain is automatically assigned and always available.
            </p>
          </div>
        )}

        {/* Legacy domain display (if set) */}
        {app.domain && domains.length === 0 && (
          <div className="space-y-2 p-3 bg-muted/50 rounded-lg border-l-4 border-yellow-500">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Badge variant="outline">Legacy Domain</Badge>
                <span className="font-mono text-sm">{app.domain}</span>
              </div>
            </div>
            <p className="text-xs text-muted-foreground">
              This is your legacy primary domain. Add domains below to use the new
              multi-domain feature.
            </p>
          </div>
        )}

        {/* Custom domains list */}
        {domains.length > 0 && (
          <div className="space-y-3">
            <div className="flex items-center justify-between gap-2">
              <Label className="text-sm font-medium">Custom Domains</Label>
              <Button
                variant="outline"
                size="sm"
                className="gap-1"
                onClick={() => domains.forEach((d) => runDnsCheck(d.domain))}
              >
                <RefreshCw className="h-3 w-3" />
                Check DNS
              </Button>
            </div>
            <div className="overflow-x-auto rounded-md border">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Domain</TableHead>
                    <TableHead className="w-[150px]">DNS status</TableHead>
                    <TableHead className="w-[150px]">WWW redirect</TableHead>
                    <TableHead className="w-[140px]">Redirect type</TableHead>
                    <TableHead className="w-[120px]">Internal port</TableHead>
                    <TableHead className="w-[110px] text-center">Search indexing</TableHead>
                    <TableHead className="w-[120px] text-right">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {domains.map((domain, index) => {
                    const dns = dnsResults[domain.domain];
                    const hasRedirect =
                      domain.www_redirect_mode === "to_www" || domain.www_redirect_mode === "to_non_www";
                    return (
                      <TableRow key={domain.domain}>
                        <TableCell>
                          <div className="flex items-center gap-2">
                            <span className="font-mono text-sm break-all">{domain.domain}</span>
                            {domain.primary && (
                              <Star className="h-4 w-4 shrink-0 text-yellow-500 fill-yellow-500" aria-label="Primary domain" />
                            )}
                            <Button aria-label="Copy to clipboard"
                              variant="ghost"
                              size="sm"
                              onClick={() => copyDomain(domain.domain)}
                              className="h-6 w-6 p-0"
                            >
                              <Copy className="h-3 w-3" />
                            </Button>
                            <Button aria-label="Open in new tab"
                              variant="ghost"
                              size="sm"
                              asChild
                              className="h-6 w-6 p-0"
                            >
                              <a
                                href={`https://${domain.domain}`}
                                target="_blank"
                                rel="noopener noreferrer"
                              >
                                <ExternalLink className="h-3 w-3" />
                              </a>
                            </Button>
                          </div>
                        </TableCell>
                        <TableCell>
                          <div className="flex items-center gap-1">
                            <DnsStatusBadge
                              result={dns === "loading" ? null : dns}
                              loading={dns === "loading"}
                            />
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => runDnsCheck(domain.domain)}
                              className="h-6 px-2 text-xs"
                              disabled={dns === "loading"}
                            >
                              Check
                            </Button>
                          </div>
                        </TableCell>
                        <TableCell>
                          <Select
                            value={domain.www_redirect_mode ?? "none"}
                            onValueChange={(value) =>
                              updateDomain(index, {
                                www_redirect_mode:
                                  value === "none" ? undefined : (value as Domain["www_redirect_mode"]),
                              })
                            }
                          >
                            <SelectTrigger aria-label={`WWW redirect for ${domain.domain}`} className="h-7 text-xs w-36">
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              <SelectItem value="none">No redirect</SelectItem>
                              <SelectItem value="both">Serve both</SelectItem>
                              <SelectItem value="to_www">To www</SelectItem>
                              <SelectItem value="to_non_www">To non-www</SelectItem>
                            </SelectContent>
                          </Select>
                        </TableCell>
                        <TableCell>
                          <Select
                            value={domain.redirect_type ?? "permanent"}
                            onValueChange={(value) =>
                              updateDomain(index, {
                                redirect_type: value === "permanent" ? undefined : (value as Domain["redirect_type"]),
                              })
                            }
                            disabled={!hasRedirect}
                          >
                            <SelectTrigger
                              aria-label={`Redirect type for ${domain.domain}`}
                              className="h-7 text-xs w-32"
                              title={hasRedirect ? undefined : "Applies when a www redirect is set"}
                            >
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              <SelectItem value="permanent">Permanent (301)</SelectItem>
                              <SelectItem value="temporary">Temporary (302)</SelectItem>
                            </SelectContent>
                          </Select>
                        </TableCell>
                        <TableCell>
                          <Input
                            aria-label={`Internal port for ${domain.domain}`}
                            type="number"
                            min={1}
                            max={65535}
                            className="h-7 w-24 text-xs"
                            placeholder={String(app.port)}
                            value={domain.internal_port ?? ""}
                            onChange={(e) =>
                              updateDomain(index, {
                                internal_port: e.target.value === "" ? undefined : Number(e.target.value),
                              })
                            }
                          />
                        </TableCell>
                        <TableCell className="text-center">
                          <Switch
                            aria-label={`Search indexing for ${domain.domain}`}
                            checked={domain.search_indexing !== false}
                            onCheckedChange={(checked) => updateDomain(index, { search_indexing: checked })}
                          />
                        </TableCell>
                        <TableCell>
                          <div className="flex items-center justify-end gap-1">
                            <Button
                              variant={domain.primary ? "secondary" : "outline"}
                              size="sm"
                              onClick={() => setPrimaryDomain(index)}
                              disabled={domain.primary}
                              className="h-7 text-xs"
                            >
                              {domain.primary ? "Primary" : "Make primary"}
                            </Button>
                            <Button aria-label={`Delete ${domain.domain}`}
                              variant="ghost"
                              size="sm"
                              onClick={() => removeDomain(index)}
                              className="h-7 w-7 p-0 text-red-500 hover:text-red-600"
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          </div>
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </div>
            <p className="text-xs text-muted-foreground">
              Internal port routes a domain to another port inside the container (empty uses the
              app port, {app.port}); a new port takes effect on the next deploy. Turning search
              indexing off adds <code className="font-mono">X-Robots-Tag: noindex, nofollow</code> to
              every response on that domain.
            </p>
          </div>
        )}

        {/* Add new domain form */}
        <div className="space-y-3 pt-4 border-t">
          <Label className="text-sm font-medium">Add Domain</Label>
          <div className="flex flex-col gap-4 sm:flex-row sm:items-start">
            <div className="flex-1 space-y-1">
              <Input aria-label="example.com"
                placeholder="example.com"
                value={newDomain.domain}
                onChange={(e) =>
                  setNewDomain({ ...newDomain, domain: e.target.value })
                }
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    addDomain();
                  }
                }}
              />
              <p className="text-xs text-muted-foreground">
                Enter a domain without the protocol (no http:// or https://)
              </p>
            </div>
            <div className="flex flex-col gap-1">
              <Label className="text-xs text-muted-foreground">WWW Redirect</Label>
              <Select
                value={newDomain.www_redirect_mode}
                onValueChange={(value) =>
                  setNewDomain({
                    ...newDomain,
                    www_redirect_mode: value as typeof newDomain.www_redirect_mode,
                  })
                }
              >
                <SelectTrigger aria-label="WWW redirect mode" className="h-9 text-sm w-36">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">No redirect</SelectItem>
                  <SelectItem value="both">Serve both</SelectItem>
                  <SelectItem value="to_www">To www</SelectItem>
                  <SelectItem value="to_non_www">To non-www</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <Button
              variant="outline"
              onClick={handleGenerateDomain}
              disabled={isGeneratingDomain}
              title="Generate a random subdomain"
            >
              <Wand2 className="h-4 w-4 mr-1" />
              {isGeneratingDomain ? "Generating..." : "Generate"}
            </Button>
            <Button
              variant="outline"
              onClick={addDomain}
              disabled={!newDomain.domain.trim()}
            >
              <Plus className="h-4 w-4 mr-1" />
              Add Domain
            </Button>
          </div>
        </div>

        {/* DNS Configuration Help */}
        {domains.length > 0 && (
          <div className="space-y-2 p-3 bg-blue-50 dark:bg-blue-950/20 rounded-lg border border-blue-200 dark:border-blue-800">
            <Label className="text-sm font-medium text-blue-800 dark:text-blue-300">
              DNS Configuration
            </Label>
            <div className="text-xs text-blue-700 dark:text-blue-400 space-y-1">
              <p>Point your domains to this server using one of these methods:</p>
              <ul className="list-disc list-inside pl-2 space-y-1">
                <li>
                  <strong>A Record:</strong> Point to your server&apos;s IP address
                </li>
                <li>
                  <strong>CNAME Record:</strong> Point to your auto-generated subdomain
                  {app.auto_subdomain && (
                    <span className="font-mono ml-1">({app.auto_subdomain})</span>
                  )}
                </li>
              </ul>
              <p className="mt-2">
                SSL certificates will be automatically provisioned when ACME is enabled.
              </p>
            </div>
          </div>
        )}
      </CardContent>
      <UnsavedChangesBar
        show={form.isDirty}
        onSave={() => void handleSave()}
        onReset={form.reset}
        saving={isSaving}
        saveDisabled={Boolean(invalidPort)}
        message="You have domain changes that haven't been saved yet."
      />
    </Card>
  );
}
