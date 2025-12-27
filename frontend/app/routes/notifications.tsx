import type { Route } from "./+types/notifications";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Bell, Mail, MessageSquare, Webhook } from "lucide-react";
import { Button } from "@/components/ui/button";

export function meta() {
  return [
    { title: "Notifications - Rivetr" },
    { name: "description", content: "Configure notification channels and preferences" },
  ];
}

export async function loader({ request }: Route.LoaderArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  await requireAuth(request);
  return null;
}

export default function NotificationsPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Notifications</h1>
        <p className="text-muted-foreground">
          Configure notification channels and preferences
        </p>
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        <Card>
          <CardHeader>
            <div className="flex items-center gap-2">
              <Mail className="h-5 w-5" />
              <CardTitle className="text-lg">Email</CardTitle>
            </div>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground mb-4">
              Receive deployment notifications via email
            </p>
            <Button variant="outline" disabled>
              Configure
            </Button>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <div className="flex items-center gap-2">
              <MessageSquare className="h-5 w-5" />
              <CardTitle className="text-lg">Slack</CardTitle>
            </div>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground mb-4">
              Send notifications to Slack channels
            </p>
            <Button variant="outline" disabled>
              Connect
            </Button>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <div className="flex items-center gap-2">
              <Webhook className="h-5 w-5" />
              <CardTitle className="text-lg">Discord</CardTitle>
            </div>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground mb-4">
              Send notifications to Discord webhooks
            </p>
            <Button variant="outline" disabled>
              Connect
            </Button>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Notification History</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex flex-col items-center justify-center py-12 text-center">
            <Bell className="h-12 w-12 text-muted-foreground mb-4" />
            <h3 className="text-lg font-medium">Notifications Coming Soon</h3>
            <p className="text-muted-foreground max-w-sm mt-2">
              Get notified about deployments, failures, and important events
              via email, Slack, Discord, and custom webhooks.
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
