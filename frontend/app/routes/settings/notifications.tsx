import { useState, useEffect } from "react";
import { Form, useNavigation } from "react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { Route } from "./+types/notifications";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
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
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { api } from "@/lib/api";
import type {
  NotificationChannel,
  NotificationChannelType,
  NotificationSubscription,
  NotificationEventType,
  App,
  SlackConfig,
  DiscordConfig,
  EmailConfig,
} from "@/types/api";
import { Loader2, Plus, Trash2, Send, Bell, MessageSquare, Mail, Check, X } from "lucide-react";

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

function getChannelIcon(type: NotificationChannelType) {
  switch (type) {
    case "slack":
      return <MessageSquare className="h-4 w-4" />;
    case "discord":
      return <Bell className="h-4 w-4" />;
    case "email":
      return <Mail className="h-4 w-4" />;
  }
}

function getChannelBadgeVariant(type: NotificationChannelType): "default" | "secondary" | "outline" {
  switch (type) {
    case "slack":
      return "default";
    case "discord":
      return "secondary";
    case "email":
      return "outline";
  }
}

const EVENT_TYPES: { value: NotificationEventType; label: string }[] = [
  { value: "deployment_started", label: "Deployment Started" },
  { value: "deployment_success", label: "Deployment Successful" },
  { value: "deployment_failed", label: "Deployment Failed" },
  { value: "app_started", label: "App Started" },
  { value: "app_stopped", label: "App Stopped" },
];

export function meta() {
  return [
    { title: "Notifications - Rivetr" },
    { name: "description", content: "Configure notification channels for deployment alerts" },
  ];
}

export async function loader({ request }: Route.LoaderArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const [channels, apps] = await Promise.all([
    api.getNotificationChannels(token).catch(() => []),
    api.getApps(token).catch(() => []),
  ]);
  return { channels, apps, token };
}

export async function action({ request }: Route.ActionArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  if (intent === "create") {
    const name = formData.get("name") as string;
    const channelType = formData.get("channel_type") as NotificationChannelType;

    if (!name?.trim()) {
      return { error: "Name is required" };
    }

    let config: SlackConfig | DiscordConfig | EmailConfig;

    if (channelType === "slack") {
      const webhookUrl = formData.get("webhook_url") as string;
      if (!webhookUrl?.trim()) {
        return { error: "Webhook URL is required" };
      }
      config = { webhook_url: webhookUrl.trim() };
    } else if (channelType === "discord") {
      const webhookUrl = formData.get("webhook_url") as string;
      if (!webhookUrl?.trim()) {
        return { error: "Webhook URL is required" };
      }
      config = { webhook_url: webhookUrl.trim() };
    } else if (channelType === "email") {
      const smtpHost = formData.get("smtp_host") as string;
      const smtpPort = parseInt(formData.get("smtp_port") as string, 10);
      const smtpUsername = formData.get("smtp_username") as string;
      const smtpPassword = formData.get("smtp_password") as string;
      const smtpTls = formData.get("smtp_tls") === "true";
      const fromAddress = formData.get("from_address") as string;
      const toAddresses = (formData.get("to_addresses") as string)
        .split(",")
        .map((a) => a.trim())
        .filter((a) => a);

      if (!smtpHost?.trim()) {
        return { error: "SMTP host is required" };
      }
      if (!smtpPort || smtpPort <= 0) {
        return { error: "Valid SMTP port is required" };
      }
      if (!fromAddress?.trim()) {
        return { error: "From address is required" };
      }
      if (toAddresses.length === 0) {
        return { error: "At least one recipient address is required" };
      }

      config = {
        smtp_host: smtpHost.trim(),
        smtp_port: smtpPort,
        smtp_username: smtpUsername?.trim() || undefined,
        smtp_password: smtpPassword || undefined,
        smtp_tls: smtpTls,
        from_address: fromAddress.trim(),
        to_addresses: toAddresses,
      };
    } else {
      return { error: "Invalid channel type" };
    }

    try {
      await api.createNotificationChannel(token, {
        name: name.trim(),
        channel_type: channelType,
        config,
        enabled: true,
      });
      return { success: true, action: "create" };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to create channel" };
    }
  }

  if (intent === "delete") {
    const channelId = formData.get("channelId") as string;
    if (!channelId) {
      return { error: "Channel ID is required" };
    }
    try {
      await api.deleteNotificationChannel(token, channelId);
      return { success: true, action: "delete" };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to delete channel" };
    }
  }

  if (intent === "toggle") {
    const channelId = formData.get("channelId") as string;
    const enabled = formData.get("enabled") === "true";
    if (!channelId) {
      return { error: "Channel ID is required" };
    }
    try {
      await api.updateNotificationChannel(token, channelId, { enabled });
      return { success: true, action: "toggle" };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to update channel" };
    }
  }

  if (intent === "test") {
    const channelId = formData.get("channelId") as string;
    if (!channelId) {
      return { error: "Channel ID is required" };
    }
    try {
      await api.testNotificationChannel(token, channelId);
      return { success: true, action: "test" };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to send test notification" };
    }
  }

  if (intent === "add_subscription") {
    const channelId = formData.get("channelId") as string;
    const eventType = formData.get("event_type") as NotificationEventType;
    const appIdRaw = formData.get("app_id") as string;
    const appId = appIdRaw && appIdRaw !== "__all__" ? appIdRaw : undefined;

    if (!channelId) {
      return { error: "Channel ID is required" };
    }
    if (!eventType) {
      return { error: "Event type is required" };
    }

    try {
      await api.createNotificationSubscription(token, channelId, {
        event_type: eventType,
        app_id: appId,
      });
      return { success: true, action: "add_subscription" };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to add subscription" };
    }
  }

  if (intent === "delete_subscription") {
    const subscriptionId = formData.get("subscriptionId") as string;
    if (!subscriptionId) {
      return { error: "Subscription ID is required" };
    }
    try {
      await api.deleteNotificationSubscription(token, subscriptionId);
      return { success: true, action: "delete_subscription" };
    } catch (error) {
      return { error: error instanceof Error ? error.message : "Failed to delete subscription" };
    }
  }

  return { error: "Unknown action" };
}

export default function SettingsNotificationsPage({ loaderData, actionData }: Route.ComponentProps) {
  const queryClient = useQueryClient();
  const navigation = useNavigation();
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [showSubscriptionsDialog, setShowSubscriptionsDialog] = useState(false);
  const [selectedChannel, setSelectedChannel] = useState<NotificationChannel | null>(null);
  const [channelType, setChannelType] = useState<NotificationChannelType>("slack");
  const [subscriptions, setSubscriptions] = useState<NotificationSubscription[]>([]);
  const [loadingSubscriptions, setLoadingSubscriptions] = useState(false);

  const { data: channels = [] } = useQuery<NotificationChannel[]>({
    queryKey: ["notification-channels"],
    queryFn: () => api.getNotificationChannels(loaderData.token),
    initialData: loaderData.channels,
  });

  const apps = loaderData.apps as App[];
  const isSubmitting = navigation.state === "submitting";

  // Handle success actions
  useEffect(() => {
    if (actionData?.success) {
      if (actionData.action === "create") {
        toast.success("Notification channel created");
        setShowCreateDialog(false);
      } else if (actionData.action === "delete") {
        toast.success("Notification channel deleted");
        setShowDeleteDialog(false);
        setSelectedChannel(null);
      } else if (actionData.action === "toggle") {
        toast.success("Channel updated");
      } else if (actionData.action === "test") {
        toast.success("Test notification sent");
      } else if (actionData.action === "add_subscription") {
        toast.success("Subscription added");
        // Reload subscriptions
        if (selectedChannel) {
          loadSubscriptions(selectedChannel.id);
        }
      } else if (actionData.action === "delete_subscription") {
        toast.success("Subscription removed");
        if (selectedChannel) {
          loadSubscriptions(selectedChannel.id);
        }
      }
      queryClient.invalidateQueries({ queryKey: ["notification-channels"] });
    }

    if (actionData?.error) {
      toast.error(actionData.error);
    }
  }, [actionData, queryClient, selectedChannel]);

  const loadSubscriptions = async (channelId: string) => {
    setLoadingSubscriptions(true);
    try {
      const subs = await api.getNotificationSubscriptions(channelId, loaderData.token);
      setSubscriptions(subs);
    } catch {
      toast.error("Failed to load subscriptions");
    } finally {
      setLoadingSubscriptions(false);
    }
  };

  const openSubscriptionsDialog = (channel: NotificationChannel) => {
    setSelectedChannel(channel);
    setShowSubscriptionsDialog(true);
    loadSubscriptions(channel.id);
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Notifications</h1>
          <p className="text-muted-foreground">
            Configure notification channels to receive alerts on deployment events
          </p>
        </div>
        <Button onClick={() => setShowCreateDialog(true)}>
          <Plus className="mr-2 h-4 w-4" />
          Add Channel
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Notification Channels</CardTitle>
          <CardDescription>
            Send notifications via Slack, Discord, or Email when deployments occur.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {channels.length === 0 ? (
            <p className="text-muted-foreground py-4 text-center">
              No notification channels configured. Add one to receive deployment alerts.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead className="w-48">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {channels.map((channel) => (
                  <TableRow key={channel.id}>
                    <TableCell className="font-medium">{channel.name}</TableCell>
                    <TableCell>
                      <Badge variant={getChannelBadgeVariant(channel.channel_type)}>
                        <span className="flex items-center gap-1">
                          {getChannelIcon(channel.channel_type)}
                          {channel.channel_type.charAt(0).toUpperCase() + channel.channel_type.slice(1)}
                        </span>
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <Form method="post">
                        <input type="hidden" name="intent" value="toggle" />
                        <input type="hidden" name="channelId" value={channel.id} />
                        <input
                          type="hidden"
                          name="enabled"
                          value={channel.enabled ? "false" : "true"}
                        />
                        <Button
                          type="submit"
                          variant="ghost"
                          size="sm"
                          className="p-0"
                          disabled={isSubmitting}
                        >
                          {channel.enabled ? (
                            <Badge variant="default" className="bg-green-600">
                              <Check className="mr-1 h-3 w-3" />
                              Enabled
                            </Badge>
                          ) : (
                            <Badge variant="secondary">
                              <X className="mr-1 h-3 w-3" />
                              Disabled
                            </Badge>
                          )}
                        </Button>
                      </Form>
                    </TableCell>
                    <TableCell>{formatDate(channel.created_at)}</TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => openSubscriptionsDialog(channel)}
                        >
                          Subscriptions
                        </Button>
                        <Form method="post" className="inline">
                          <input type="hidden" name="intent" value="test" />
                          <input type="hidden" name="channelId" value={channel.id} />
                          <Button
                            type="submit"
                            variant="outline"
                            size="sm"
                            disabled={isSubmitting || !channel.enabled}
                          >
                            <Send className="h-4 w-4" />
                          </Button>
                        </Form>
                        <Button
                          variant="destructive"
                          size="sm"
                          onClick={() => {
                            setSelectedChannel(channel);
                            setShowDeleteDialog(true);
                          }}
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Create Channel Dialog */}
      <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
        <DialogContent className="max-w-lg">
          <Form method="post">
            <input type="hidden" name="intent" value="create" />
            <input type="hidden" name="channel_type" value={channelType} />
            <DialogHeader>
              <DialogTitle>Add Notification Channel</DialogTitle>
              <DialogDescription>
                Configure a new channel to receive deployment notifications.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label htmlFor="name">Channel Name</Label>
                <Input
                  id="name"
                  name="name"
                  placeholder="e.g., Production Alerts"
                  required
                />
              </div>

              <div className="space-y-2">
                <Label>Channel Type</Label>
                <Select value={channelType} onValueChange={(v) => setChannelType(v as NotificationChannelType)}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="slack">
                      <span className="flex items-center gap-2">
                        <MessageSquare className="h-4 w-4" />
                        Slack
                      </span>
                    </SelectItem>
                    <SelectItem value="discord">
                      <span className="flex items-center gap-2">
                        <Bell className="h-4 w-4" />
                        Discord
                      </span>
                    </SelectItem>
                    <SelectItem value="email">
                      <span className="flex items-center gap-2">
                        <Mail className="h-4 w-4" />
                        Email (SMTP)
                      </span>
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {/* Slack Config */}
              {channelType === "slack" && (
                <div className="space-y-2">
                  <Label htmlFor="webhook_url">Webhook URL</Label>
                  <Input
                    id="webhook_url"
                    name="webhook_url"
                    placeholder="https://hooks.slack.com/services/..."
                    required
                  />
                  <p className="text-xs text-muted-foreground">
                    Get this from your Slack App's Incoming Webhooks settings.
                  </p>
                </div>
              )}

              {/* Discord Config */}
              {channelType === "discord" && (
                <div className="space-y-2">
                  <Label htmlFor="webhook_url">Webhook URL</Label>
                  <Input
                    id="webhook_url"
                    name="webhook_url"
                    placeholder="https://discord.com/api/webhooks/..."
                    required
                  />
                  <p className="text-xs text-muted-foreground">
                    Get this from your Discord channel's Integrations settings.
                  </p>
                </div>
              )}

              {/* Email Config */}
              {channelType === "email" && (
                <>
                  <div className="grid grid-cols-2 gap-4">
                    <div className="space-y-2">
                      <Label htmlFor="smtp_host">SMTP Host</Label>
                      <Input
                        id="smtp_host"
                        name="smtp_host"
                        placeholder="smtp.example.com"
                        required
                      />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="smtp_port">SMTP Port</Label>
                      <Input
                        id="smtp_port"
                        name="smtp_port"
                        type="number"
                        placeholder="587"
                        defaultValue="587"
                        required
                      />
                    </div>
                  </div>
                  <div className="grid grid-cols-2 gap-4">
                    <div className="space-y-2">
                      <Label htmlFor="smtp_username">Username (optional)</Label>
                      <Input
                        id="smtp_username"
                        name="smtp_username"
                        placeholder="user@example.com"
                      />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="smtp_password">Password (optional)</Label>
                      <Input
                        id="smtp_password"
                        name="smtp_password"
                        type="password"
                        placeholder="********"
                      />
                    </div>
                  </div>
                  <div className="flex items-center space-x-2">
                    <Switch id="smtp_tls" name="smtp_tls" defaultChecked />
                    <Label htmlFor="smtp_tls">Use TLS</Label>
                    <input type="hidden" name="smtp_tls" value="true" />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="from_address">From Address</Label>
                    <Input
                      id="from_address"
                      name="from_address"
                      placeholder="noreply@example.com"
                      required
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="to_addresses">To Addresses (comma-separated)</Label>
                    <Input
                      id="to_addresses"
                      name="to_addresses"
                      placeholder="admin@example.com, devops@example.com"
                      required
                    />
                  </div>
                </>
              )}
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => setShowCreateDialog(false)}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Creating...
                  </>
                ) : (
                  "Create Channel"
                )}
              </Button>
            </DialogFooter>
          </Form>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <Dialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Notification Channel</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete "{selectedChannel?.name}"? This will also remove all
              associated subscriptions. This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setShowDeleteDialog(false);
                setSelectedChannel(null);
              }}
            >
              Cancel
            </Button>
            <Form method="post">
              <input type="hidden" name="intent" value="delete" />
              <input type="hidden" name="channelId" value={selectedChannel?.id || ""} />
              <Button type="submit" variant="destructive" disabled={isSubmitting}>
                {isSubmitting ? "Deleting..." : "Delete"}
              </Button>
            </Form>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Subscriptions Dialog */}
      <Dialog open={showSubscriptionsDialog} onOpenChange={setShowSubscriptionsDialog}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Manage Subscriptions</DialogTitle>
            <DialogDescription>
              Configure which events trigger notifications for "{selectedChannel?.name}".
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            {/* Add Subscription Form */}
            <Form method="post" className="flex items-end gap-4">
              <input type="hidden" name="intent" value="add_subscription" />
              <input type="hidden" name="channelId" value={selectedChannel?.id || ""} />
              <div className="flex-1 space-y-2">
                <Label>Event Type</Label>
                <Select name="event_type" required>
                  <SelectTrigger>
                    <SelectValue placeholder="Select event type" />
                  </SelectTrigger>
                  <SelectContent>
                    {EVENT_TYPES.map((event) => (
                      <SelectItem key={event.value} value={event.value}>
                        {event.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="flex-1 space-y-2">
                <Label>App (optional)</Label>
                <Select name="app_id" defaultValue="__all__">
                  <SelectTrigger>
                    <SelectValue placeholder="All apps" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="__all__">All Apps</SelectItem>
                    {apps.filter((app) => app.id).map((app) => (
                      <SelectItem key={app.id} value={app.id}>
                        {app.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <Button type="submit" disabled={isSubmitting}>
                <Plus className="mr-2 h-4 w-4" />
                Add
              </Button>
            </Form>

            {/* Subscriptions List */}
            <div className="border rounded-md">
              {loadingSubscriptions ? (
                <div className="flex items-center justify-center py-8">
                  <Loader2 className="h-6 w-6 animate-spin" />
                </div>
              ) : subscriptions.length === 0 ? (
                <p className="text-muted-foreground py-8 text-center">
                  No subscriptions configured. Add one above.
                </p>
              ) : (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Event Type</TableHead>
                      <TableHead>App</TableHead>
                      <TableHead className="w-24">Actions</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {subscriptions.map((sub) => (
                      <TableRow key={sub.id}>
                        <TableCell>
                          {EVENT_TYPES.find((e) => e.value === sub.event_type)?.label ||
                            sub.event_type}
                        </TableCell>
                        <TableCell>{sub.app_name || "All Apps"}</TableCell>
                        <TableCell>
                          <Form method="post">
                            <input type="hidden" name="intent" value="delete_subscription" />
                            <input type="hidden" name="subscriptionId" value={sub.id} />
                            <Button
                              type="submit"
                              variant="destructive"
                              size="sm"
                              disabled={isSubmitting}
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          </Form>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              )}
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowSubscriptionsDialog(false)}>
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
