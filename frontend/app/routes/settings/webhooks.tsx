import type { Route } from "./+types/webhooks";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

export async function loader({ request }: Route.LoaderArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  await requireAuth(request);
  return null;
}

export default function SettingsWebhooksPage() {
  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold">Webhooks</h1>

      <Card>
        <CardHeader>
          <CardTitle>Webhook Endpoints</CardTitle>
          <CardDescription>
            Configure your Git providers to send webhooks to these endpoints for automatic deployments.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <span className="font-medium">GitHub</span>
              <Badge variant="outline">Supported</Badge>
            </div>
            <code className="block rounded bg-muted px-3 py-2 text-sm">
              POST /webhooks/github
            </code>
            <p className="text-xs text-muted-foreground">
              Set this URL as your GitHub webhook endpoint. Use application/json content type.
            </p>
          </div>

          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <span className="font-medium">GitLab</span>
              <Badge variant="outline">Supported</Badge>
            </div>
            <code className="block rounded bg-muted px-3 py-2 text-sm">
              POST /webhooks/gitlab
            </code>
            <p className="text-xs text-muted-foreground">
              Set this URL as your GitLab webhook endpoint for push events.
            </p>
          </div>

          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <span className="font-medium">Gitea</span>
              <Badge variant="outline">Supported</Badge>
            </div>
            <code className="block rounded bg-muted px-3 py-2 text-sm">
              POST /webhooks/gitea
            </code>
            <p className="text-xs text-muted-foreground">
              Set this URL as your Gitea webhook endpoint for push events.
            </p>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Webhook Security</CardTitle>
          <CardDescription>
            Configure webhook secret validation.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            Webhook signature verification is not yet implemented.
            Coming in a future update.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
