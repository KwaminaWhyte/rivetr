import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Globe, Info } from "lucide-react";

export function meta() {
  return [
    { title: "Settings - Rivetr" },
    { name: "description", content: "Configure your Rivetr instance settings" },
  ];
}

export default function SettingsPage() {
  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold">Settings</h1>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Globe className="h-5 w-5" />
            Instance Domain
          </CardTitle>
          <CardDescription>
            Configure a custom domain so users can access the Rivetr dashboard through it.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-start gap-2 rounded-md bg-muted p-4 text-sm">
            <Info className="h-4 w-4 mt-0.5 shrink-0 text-muted-foreground" />
            <div className="space-y-2">
              <p className="font-medium">How to configure an instance domain</p>
              <p className="text-muted-foreground">
                Add the following to your <code className="bg-background px-1 rounded font-mono">rivetr.toml</code>:
              </p>
              <pre className="bg-background rounded p-3 font-mono text-xs overflow-x-auto">
{`[proxy]
instance_domain = "rivetr.yourdomain.com"`}
              </pre>
              <p className="text-muted-foreground">
                Then point your DNS A record for <code className="bg-background px-1 rounded font-mono">rivetr.yourdomain.com</code>{" "}
                to this server and restart Rivetr. The proxy will automatically forward traffic for that
                domain to the Rivetr API server, making the dashboard accessible at that address.
              </p>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Container Runtime</CardTitle>
          <CardDescription>
            Information about the active container runtime.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-1">
              <Label className="text-muted-foreground text-xs">Runtime</Label>
              <p className="font-medium">Docker / Podman (auto-detected)</p>
            </div>
            <div className="space-y-1">
              <Label className="text-muted-foreground text-xs">Status</Label>
              <p className="font-medium text-green-600">Connected</p>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
