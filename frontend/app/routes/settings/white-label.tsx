import { useState, useEffect } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Save, Paintbrush } from "lucide-react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Separator } from "@/components/ui/separator";
import { whiteLabelApi } from "@/lib/api/white-label";
import type { WhiteLabel, UpdateWhiteLabelRequest } from "@/lib/api/white-label";

export function meta() {
  return [
    { title: "White Label - Settings" },
    { name: "description", content: "Customize your Rivetr instance branding" },
  ];
}

export default function WhiteLabelSettingsPage() {
  const queryClient = useQueryClient();

  const { data: config, isLoading } = useQuery<WhiteLabel>({
    queryKey: ["white-label"],
    queryFn: whiteLabelApi.get,
  });

  const [form, setForm] = useState<UpdateWhiteLabelRequest>({
    app_name: "Rivetr",
    app_description: null,
    logo_url: null,
    favicon_url: null,
    custom_css: null,
    footer_text: null,
    support_url: null,
    docs_url: null,
    login_page_message: null,
  });

  // Sync form when data loads
  useEffect(() => {
    if (config) {
      setForm({
        app_name: config.app_name,
        app_description: config.app_description ?? null,
        logo_url: config.logo_url ?? null,
        favicon_url: config.favicon_url ?? null,
        custom_css: config.custom_css ?? null,
        footer_text: config.footer_text ?? null,
        support_url: config.support_url ?? null,
        docs_url: config.docs_url ?? null,
        login_page_message: config.login_page_message ?? null,
      });
    }
  }, [config]);

  const updateMutation = useMutation({
    mutationFn: (data: UpdateWhiteLabelRequest) => whiteLabelApi.update(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["white-label"] });
      toast.success("White label settings saved");
    },
    onError: () => toast.error("Failed to save white label settings"),
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    updateMutation.mutate(form);
  };

  const set = (key: keyof UpdateWhiteLabelRequest) =>
    (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) =>
      setForm((prev) => ({ ...prev, [key]: e.target.value || null }));

  const previewName = form.app_name || "Rivetr";
  const previewLogo = form.logo_url;

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold flex items-center gap-2">
          <Paintbrush className="h-7 w-7" />
          White Label
        </h1>
        <p className="text-muted-foreground">
          Customize the branding and appearance of your Rivetr instance.
        </p>
      </div>

      <form onSubmit={handleSubmit} className="space-y-6">
        {/* Identity */}
        <Card>
          <CardHeader>
            <CardTitle>Identity</CardTitle>
            <CardDescription>
              Set the name and description for your instance.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="app_name">App Name</Label>
              <Input
                id="app_name"
                placeholder="Rivetr"
                value={form.app_name ?? ""}
                onChange={(e) =>
                  setForm((prev) => ({ ...prev, app_name: e.target.value || "Rivetr" }))
                }
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="app_description">App Description</Label>
              <Input
                id="app_description"
                placeholder="Deploy apps with ease"
                value={form.app_description ?? ""}
                onChange={set("app_description")}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="login_page_message">Login Page Message</Label>
              <Textarea
                id="login_page_message"
                placeholder="Welcome! Sign in to manage your deployments."
                rows={2}
                value={form.login_page_message ?? ""}
                onChange={set("login_page_message")}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="footer_text">Footer Text</Label>
              <Input
                id="footer_text"
                placeholder="© 2025 My Company"
                value={form.footer_text ?? ""}
                onChange={set("footer_text")}
              />
            </div>
          </CardContent>
        </Card>

        {/* Branding assets */}
        <Card>
          <CardHeader>
            <CardTitle>Logo & Favicon</CardTitle>
            <CardDescription>
              Provide URLs to your logo and favicon images.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="logo_url">Logo URL</Label>
              <Input
                id="logo_url"
                placeholder="https://example.com/logo.png"
                value={form.logo_url ?? ""}
                onChange={set("logo_url")}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="favicon_url">Favicon URL</Label>
              <Input
                id="favicon_url"
                placeholder="https://example.com/favicon.ico"
                value={form.favicon_url ?? ""}
                onChange={set("favicon_url")}
              />
            </div>
          </CardContent>
        </Card>

        {/* Custom CSS */}
        <Card>
          <CardHeader>
            <CardTitle>Custom CSS</CardTitle>
            <CardDescription>
              Applied globally via a{" "}
              <code className="text-xs bg-muted px-1 rounded">&lt;style&gt;</code>{" "}
              tag injected into the page. Use with care.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Textarea
              id="custom_css"
              placeholder={`:root {\n  --primary: 220 90% 56%;\n}`}
              rows={8}
              className="font-mono text-sm"
              value={form.custom_css ?? ""}
              onChange={set("custom_css")}
            />
          </CardContent>
        </Card>

        {/* Links */}
        <Card>
          <CardHeader>
            <CardTitle>Links</CardTitle>
            <CardDescription>
              Optional support and documentation URLs shown in the UI.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="support_url">Support URL</Label>
              <Input
                id="support_url"
                placeholder="https://support.example.com"
                value={form.support_url ?? ""}
                onChange={set("support_url")}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="docs_url">Docs URL</Label>
              <Input
                id="docs_url"
                placeholder="https://docs.example.com"
                value={form.docs_url ?? ""}
                onChange={set("docs_url")}
              />
            </div>
          </CardContent>
        </Card>

        {/* Live preview */}
        <Card>
          <CardHeader>
            <CardTitle>Sidebar Preview</CardTitle>
            <CardDescription>
              How the sidebar header will appear with your branding.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-3 rounded-md border p-3 bg-muted/30 w-fit">
              {previewLogo ? (
                <img
                  src={previewLogo}
                  alt={previewName}
                  className="h-6 w-auto object-contain"
                  onError={(e) => {
                    (e.target as HTMLImageElement).style.display = "none";
                  }}
                />
              ) : (
                <div className="h-6 w-6 rounded bg-primary/20 flex items-center justify-center text-xs font-bold text-primary">
                  {previewName.slice(0, 2).toUpperCase()}
                </div>
              )}
              <span className="font-semibold text-sm">{previewName}</span>
            </div>
          </CardContent>
        </Card>

        <Separator />

        <div className="flex justify-end">
          <Button type="submit" disabled={updateMutation.isPending || isLoading}>
            <Save className="h-4 w-4 mr-2" />
            {updateMutation.isPending ? "Saving…" : "Save Changes"}
          </Button>
        </div>
      </form>
    </div>
  );
}
