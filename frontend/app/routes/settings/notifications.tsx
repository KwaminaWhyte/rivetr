import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
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

export default function SettingsNotificationsPage() {
  const queryClient = useQueryClient();
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [showSubscriptionsDialog, setShowSubscriptionsDialog] = useState(false);
  const [selectedChannel, setSelectedChannel] = useState<NotificationChannel | null>(null);
  const [channelType, setChannelType] = useState<NotificationChannelType>("slack");
  const [subscriptions, setSubscriptions] = useState<NotificationSubscription[]>([]);
  const [loadingSubscriptions, setLoadingSubscriptions] = useState(false);

  // Form state for create dialog
  const [formName, setFormName] = useState("");
  const [webhookUrl, setWebhookUrl] = useState("");
  const [smtpHost, setSmtpHost] = useState("");
  const [smtpPort, setSmtpPort] = useState("587");
  const [smtpUsername, setSmtpUsername] = useState("");
  const [smtpPassword, setSmtpPassword] = useState("");
  const [smtpTls, setSmtpTls] = useState(true);
  const [fromAddress, setFromAddress] = useState("");
  const [toAddresses, setToAddresses] = useState("");

  // Subscription form state
  const [subEventType, setSubEventType] = useState<NotificationEventType | "">("");
  const [subAppId, setSubAppId] = useState<string>("__all__");

  const { data: channels = [], isLoading: channelsLoading } = useQuery<NotificationChannel[]>({
    queryKey: ["notification-channels"],
    queryFn: () => api.getNotificationChannels(),
  });

  const { data: apps = [] } = useQuery<App[]>({
    queryKey: ["apps"],
    queryFn: () => api.getApps(),
  });

  const createMutation = useMutation({
    mutationFn: async () => {
      let config: SlackConfig | DiscordConfig | EmailConfig;

      if (channelType === "slack") {
        config = { webhook_url: webhookUrl.trim() };
      } else if (channelType === "discord") {
        config = { webhook_url: webhookUrl.trim() };
      } else {
        const addresses = toAddresses
          .split(",")
          .map((a) => a.trim())
          .filter((a) => a);
        config = {
          smtp_host: smtpHost.trim(),
          smtp_port: parseInt(smtpPort, 10),
          smtp_username: smtpUsername.trim() || undefined,
          smtp_password: smtpPassword || undefined,
          smtp_tls: smtpTls,
          from_address: fromAddress.trim(),
          to_addresses: addresses,
        };
      }

      return api.createNotificationChannel({
        name: formName.trim(),
        channel_type: channelType,
        config,
        enabled: true,
      });
    },
    onSuccess: () => {
      toast.success("Notification channel created");
      queryClient.invalidateQueries({ queryKey: ["notification-channels"] });
      setShowCreateDialog(false);
      resetCreateForm();
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to create channel");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (channelId: string) => api.deleteNotificationChannel(channelId),
    onSuccess: () => {
      toast.success("Notification channel deleted");
      queryClient.invalidateQueries({ queryKey: ["notification-channels"] });
      setShowDeleteDialog(false);
      setSelectedChannel(null);
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to delete channel");
    },
  });

  const toggleMutation = useMutation({
    mutationFn: ({ channelId, enabled }: { channelId: string; enabled: boolean }) =>
      api.updateNotificationChannel(channelId, { enabled }),
    onSuccess: () => {
      toast.success("Channel updated");
      queryClient.invalidateQueries({ queryKey: ["notification-channels"] });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to update channel");
    },
  });

  const testMutation = useMutation({
    mutationFn: (channelId: string) => api.testNotificationChannel(channelId),
    onSuccess: () => {
      toast.success("Test notification sent");
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to send test notification");
    },
  });

  const addSubscriptionMutation = useMutation({
    mutationFn: ({ channelId, eventType, appId }: { channelId: string; eventType: NotificationEventType; appId?: string }) =>
      api.createNotificationSubscription(channelId, {
        event_type: eventType,
        app_id: appId,
      }),
    onSuccess: () => {
      toast.success("Subscription added");
      if (selectedChannel) {
        loadSubscriptions(selectedChannel.id);
      }
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to add subscription");
    },
  });

  const deleteSubscriptionMutation = useMutation({
    mutationFn: (subscriptionId: string) => api.deleteNotificationSubscription(subscriptionId),
    onSuccess: () => {
      toast.success("Subscription removed");
      if (selectedChannel) {
        loadSubscriptions(selectedChannel.id);
      }
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to delete subscription");
    },
  });

  const resetCreateForm = () => {
    setFormName("");
    setWebhookUrl("");
    setSmtpHost("");
    setSmtpPort("587");
    setSmtpUsername("");
    setSmtpPassword("");
    setSmtpTls(true);
    setFromAddress("");
    setToAddresses("");
    setChannelType("slack");
  };

  const loadSubscriptions = async (channelId: string) => {
    setLoadingSubscriptions(true);
    try {
      const subs = await api.getNotificationSubscriptions(channelId);
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

  const handleCreateSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!formName.trim()) {
      toast.error("Name is required");
      return;
    }

    if (channelType === "slack" || channelType === "discord") {
      if (!webhookUrl.trim()) {
        toast.error("Webhook URL is required");
        return;
      }
    } else if (channelType === "email") {
      if (!smtpHost.trim()) {
        toast.error("SMTP host is required");
        return;
      }
      const port = parseInt(smtpPort, 10);
      if (!port || port <= 0) {
        toast.error("Valid SMTP port is required");
        return;
      }
      if (!fromAddress.trim()) {
        toast.error("From address is required");
        return;
      }
      const addresses = toAddresses.split(",").map((a) => a.trim()).filter((a) => a);
      if (addresses.length === 0) {
        toast.error("At least one recipient address is required");
        return;
      }
    }

    createMutation.mutate();
  };

  const handleAddSubscription = (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedChannel || !subEventType) {
      toast.error("Event type is required");
      return;
    }
    addSubscriptionMutation.mutate({
      channelId: selectedChannel.id,
      eventType: subEventType as NotificationEventType,
      appId: subAppId !== "__all__" ? subAppId : undefined,
    });
    setSubEventType("");
    setSubAppId("__all__");
  };

  const isSubmitting = createMutation.isPending || deleteMutation.isPending || toggleMutation.isPending || testMutation.isPending;

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
          {channelsLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin" />
            </div>
          ) : channels.length === 0 ? (
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
                      <Button
                        variant="ghost"
                        size="sm"
                        className="p-0"
                        disabled={isSubmitting}
                        onClick={() => toggleMutation.mutate({ channelId: channel.id, enabled: !channel.enabled })}
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
                        <Button
                          variant="outline"
                          size="sm"
                          disabled={isSubmitting || !channel.enabled}
                          onClick={() => testMutation.mutate(channel.id)}
                        >
                          <Send className="h-4 w-4" />
                        </Button>
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
      <Dialog open={showCreateDialog} onOpenChange={(open) => {
        setShowCreateDialog(open);
        if (!open) resetCreateForm();
      }}>
        <DialogContent className="max-w-lg">
          <form onSubmit={handleCreateSubmit}>
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
                  value={formName}
                  onChange={(e) => setFormName(e.target.value)}
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
                    value={webhookUrl}
                    onChange={(e) => setWebhookUrl(e.target.value)}
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
                    value={webhookUrl}
                    onChange={(e) => setWebhookUrl(e.target.value)}
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
                        value={smtpHost}
                        onChange={(e) => setSmtpHost(e.target.value)}
                        placeholder="smtp.example.com"
                        required
                      />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="smtp_port">SMTP Port</Label>
                      <Input
                        id="smtp_port"
                        type="number"
                        value={smtpPort}
                        onChange={(e) => setSmtpPort(e.target.value)}
                        placeholder="587"
                        required
                      />
                    </div>
                  </div>
                  <div className="grid grid-cols-2 gap-4">
                    <div className="space-y-2">
                      <Label htmlFor="smtp_username">Username (optional)</Label>
                      <Input
                        id="smtp_username"
                        value={smtpUsername}
                        onChange={(e) => setSmtpUsername(e.target.value)}
                        placeholder="user@example.com"
                      />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="smtp_password">Password (optional)</Label>
                      <Input
                        id="smtp_password"
                        type="password"
                        value={smtpPassword}
                        onChange={(e) => setSmtpPassword(e.target.value)}
                        placeholder="********"
                      />
                    </div>
                  </div>
                  <div className="flex items-center space-x-2">
                    <Switch
                      id="smtp_tls"
                      checked={smtpTls}
                      onCheckedChange={setSmtpTls}
                    />
                    <Label htmlFor="smtp_tls">Use TLS</Label>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="from_address">From Address</Label>
                    <Input
                      id="from_address"
                      value={fromAddress}
                      onChange={(e) => setFromAddress(e.target.value)}
                      placeholder="noreply@example.com"
                      required
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="to_addresses">To Addresses (comma-separated)</Label>
                    <Input
                      id="to_addresses"
                      value={toAddresses}
                      onChange={(e) => setToAddresses(e.target.value)}
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
              <Button type="submit" disabled={createMutation.isPending}>
                {createMutation.isPending ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Creating...
                  </>
                ) : (
                  "Create Channel"
                )}
              </Button>
            </DialogFooter>
          </form>
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
            <Button
              variant="destructive"
              disabled={deleteMutation.isPending}
              onClick={() => selectedChannel && deleteMutation.mutate(selectedChannel.id)}
            >
              {deleteMutation.isPending ? "Deleting..." : "Delete"}
            </Button>
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
            <form onSubmit={handleAddSubscription} className="flex items-end gap-4">
              <div className="flex-1 space-y-2">
                <Label>Event Type</Label>
                <Select value={subEventType} onValueChange={(v) => setSubEventType(v as NotificationEventType)}>
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
                <Select value={subAppId} onValueChange={setSubAppId}>
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
              <Button type="submit" disabled={addSubscriptionMutation.isPending}>
                <Plus className="mr-2 h-4 w-4" />
                Add
              </Button>
            </form>

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
                          <Button
                            variant="destructive"
                            size="sm"
                            disabled={deleteSubscriptionMutation.isPending}
                            onClick={() => deleteSubscriptionMutation.mutate(sub.id)}
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
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
