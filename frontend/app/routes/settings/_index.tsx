import type { Route } from "./+types/_index";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";

export function meta() {
  return [
    { title: "Settings - Rivetr" },
    { name: "description", content: "Configure your Rivetr instance settings" },
  ];
}

export async function loader({ request }: Route.LoaderArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  await requireAuth(request);
  return null;
}

export default function SettingsPage() {
  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold">Settings</h1>

      <Card>
        <CardHeader>
          <CardTitle>General Settings</CardTitle>
          <CardDescription>
            Configure general settings for your Rivetr instance.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="instance-name">Instance Name</Label>
            <Input
              id="instance-name"
              placeholder="My Rivetr Instance"
              defaultValue="Rivetr"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="default-branch">Default Branch</Label>
            <Input
              id="default-branch"
              placeholder="main"
              defaultValue="main"
            />
          </div>
          <Button disabled>Save Changes</Button>
          <p className="text-xs text-muted-foreground">
            Settings configuration coming in a future update.
          </p>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Container Runtime</CardTitle>
          <CardDescription>
            View information about the detected container runtime.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4 md:grid-cols-2">
            <div>
              <div className="text-sm text-muted-foreground">Runtime</div>
              <div className="font-medium">Docker</div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">Status</div>
              <div className="font-medium text-green-600">Connected</div>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
