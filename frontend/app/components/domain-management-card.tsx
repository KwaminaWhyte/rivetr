import { useState, useEffect } from "react";
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
import { Switch } from "@/components/ui/switch";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Plus, Trash2, Globe, Star, ExternalLink, Copy } from "lucide-react";
import type { Domain, App, UpdateAppRequest } from "@/types/api";

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
  // Parse current domains from app
  const [domains, setDomains] = useState<Domain[]>(parseDomains(app.domains));

  // Form state for adding new domain
  const [newDomain, setNewDomain] = useState({
    domain: "",
    primary: false,
    redirect_www: false,
  });

  // Track if there are unsaved changes
  const [hasChanges, setHasChanges] = useState(false);

  // Update state when app changes
  useEffect(() => {
    setDomains(parseDomains(app.domains));
    setHasChanges(false);
  }, [app.domains]);

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
      redirect_www: newDomain.redirect_www,
    };

    setDomains([...updatedDomains, newEntry]);
    setNewDomain({ domain: "", primary: false, redirect_www: false });
    setHasChanges(true);
  };

  // Remove domain
  const removeDomain = (index: number) => {
    const removedDomain = domains[index];
    const newDomains = domains.filter((_, i) => i !== index);

    // If we removed the primary domain, make the first remaining domain primary
    if (removedDomain.primary && newDomains.length > 0) {
      newDomains[0].primary = true;
    }

    setDomains(newDomains);
    setHasChanges(true);
  };

  // Set domain as primary
  const setPrimaryDomain = (index: number) => {
    const updatedDomains = domains.map((d, i) => ({
      ...d,
      primary: i === index,
    }));
    setDomains(updatedDomains);
    setHasChanges(true);
  };

  // Toggle www redirect
  const toggleWwwRedirect = (index: number) => {
    const updatedDomains = [...domains];
    updatedDomains[index].redirect_www = !updatedDomains[index].redirect_www;
    setDomains(updatedDomains);
    setHasChanges(true);
  };

  // Copy domain to clipboard
  const copyDomain = (domain: string) => {
    navigator.clipboard.writeText(domain);
    toast.success("Domain copied to clipboard");
  };

  // Save handler
  const handleSave = async () => {
    try {
      await onSave({
        domains: domains,
      });
      setHasChanges(false);
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
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => copyDomain(app.auto_subdomain!)}
                  className="h-7 w-7 p-0"
                >
                  <Copy className="h-4 w-4" />
                </Button>
                <Button
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
            <Label className="text-sm font-medium">Custom Domains</Label>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Domain</TableHead>
                  <TableHead className="w-[100px] text-center">Primary</TableHead>
                  <TableHead className="w-[120px] text-center">WWW Redirect</TableHead>
                  <TableHead className="w-[80px]"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {domains.map((domain, index) => (
                  <TableRow key={index}>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <span className="font-mono text-sm">{domain.domain}</span>
                        {domain.primary && (
                          <Star className="h-4 w-4 text-yellow-500 fill-yellow-500" />
                        )}
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => copyDomain(domain.domain)}
                          className="h-6 w-6 p-0"
                        >
                          <Copy className="h-3 w-3" />
                        </Button>
                        <Button
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
                    <TableCell className="text-center">
                      <Button
                        variant={domain.primary ? "default" : "outline"}
                        size="sm"
                        onClick={() => setPrimaryDomain(index)}
                        disabled={domain.primary}
                        className="h-7"
                      >
                        {domain.primary ? "Primary" : "Set Primary"}
                      </Button>
                    </TableCell>
                    <TableCell className="text-center">
                      <Switch
                        checked={domain.redirect_www}
                        onCheckedChange={() => toggleWwwRedirect(index)}
                      />
                    </TableCell>
                    <TableCell>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => removeDomain(index)}
                        className="h-7 w-7 p-0 text-red-500 hover:text-red-600"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}

        {/* Add new domain form */}
        <div className="space-y-3 pt-4 border-t">
          <Label className="text-sm font-medium">Add Domain</Label>
          <div className="flex gap-4 items-start">
            <div className="flex-1 space-y-1">
              <Input
                placeholder="example.com"
                value={newDomain.domain}
                onChange={(e) =>
                  setNewDomain({ ...newDomain, domain: e.target.value })
                }
                onKeyDown={(e) => e.key === "Enter" && addDomain()}
              />
              <p className="text-xs text-muted-foreground">
                Enter a domain without the protocol (no http:// or https://)
              </p>
            </div>
            <div className="flex flex-col gap-2">
              <div className="flex items-center gap-2">
                <Switch
                  id="new-redirect-www"
                  checked={newDomain.redirect_www}
                  onCheckedChange={(checked) =>
                    setNewDomain({ ...newDomain, redirect_www: checked })
                  }
                />
                <Label htmlFor="new-redirect-www" className="text-xs">
                  Redirect www
                </Label>
              </div>
            </div>
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

        {/* Save Button */}
        {hasChanges && (
          <div className="flex justify-end pt-4 border-t">
            <Button onClick={handleSave} disabled={isSaving}>
              {isSaving ? "Saving..." : "Save Domain Configuration"}
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
