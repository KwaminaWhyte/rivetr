import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
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
import { Textarea } from "@/components/ui/textarea";
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
  TeamNotificationChannel,
  TeamNotificationChannelType,
  SlackConfig,
  DiscordConfig,
  EmailConfig,
  WebhookConfig,
} from "@/types/api";
import {
  Loader2,
  Plus,
  Trash2,
  Send,
  Bell,
  MessageSquare,
  Mail,
  Webhook,
  Check,
  X,
} from "lucide-react";

interface TeamNotificationChannelsCardProps {
  teamId: string;
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

function getChannelIcon(type: TeamNotificationChannelType) {
  switch (type) {
    case "slack":
      return <MessageSquare className="h-4 w-4" />;
    case "discord":
      return <Bell className="h-4 w-4" />;
    case "email":
      return <Mail className="h-4 w-4" />;
    case "webhook":
      return <Webhook className="h-4 w-4" />;
  }
}

function getChannelBadgeVariant(
  type: TeamNotificationChannelType
): "default" | "secondary" | "outline" | "destructive" {
  switch (type) {
    case "slack":
      return "default";
    case "discord":
      return "secondary";
    case "email":
      return "outline";
    case "webhook":
      return "destructive";
  }
}

const PAYLOAD_TEMPLATES: {
  value: "json" | "slack" | "discord" | "custom";
  label: string;
  description: string;
}[] = [
  { value: "json", label: "JSON", description: "Standard JSON payload" },
  { value: "slack", label: "Slack", description: "Slack webhook format" },
  { value: "discord", label: "Discord", description: "Discord webhook format" },
  {
    value: "custom",
    label: "Custom",
    description: "Custom template with variables",
  },
];

export function TeamNotificationChannelsCard({
  teamId,
}: TeamNotificationChannelsCardProps) {
  const queryClient = useQueryClient();
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [selectedChannel, setSelectedChannel] =
    useState<TeamNotificationChannel | null>(null);
  const [channelType, setChannelType] =
    useState<TeamNotificationChannelType>("slack");

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
  // Webhook-specific state
  const [payloadTemplate, setPayloadTemplate] = useState<
    "json" | "slack" | "discord" | "custom"
  >("json");
  const [customTemplate, setCustomTemplate] = useState("");

  const { data: channels = [], isLoading: channelsLoading } = useQuery<
    TeamNotificationChannel[]
  >({
    queryKey: ["team-notification-channels", teamId],
    queryFn: () => api.getTeamNotificationChannels(teamId),
  });

  const createMutation = useMutation({
    mutationFn: async () => {
      let config: SlackConfig | DiscordConfig | EmailConfig | WebhookConfig;

      if (channelType === "slack") {
        config = { webhook_url: webhookUrl.trim() };
      } else if (channelType === "discord") {
        config = { webhook_url: webhookUrl.trim() };
      } else if (channelType === "webhook") {
        config = {
          url: webhookUrl.trim(),
          payload_template: payloadTemplate,
          custom_template:
            payloadTemplate === "custom" ? customTemplate : undefined,
        };
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

      return api.createTeamNotificationChannel(teamId, {
        name: formName.trim(),
        channel_type: channelType,
        config,
        enabled: true,
      });
    },
    onSuccess: () => {
      toast.success("Notification channel created");
      queryClient.invalidateQueries({
        queryKey: ["team-notification-channels", teamId],
      });
      setShowCreateDialog(false);
      resetCreateForm();
    },
    onError: (error) => {
      toast.error(
        error instanceof Error ? error.message : "Failed to create channel"
      );
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (channelId: string) =>
      api.deleteTeamNotificationChannel(teamId, channelId),
    onSuccess: () => {
      toast.success("Notification channel deleted");
      queryClient.invalidateQueries({
        queryKey: ["team-notification-channels", teamId],
      });
      setShowDeleteDialog(false);
      setSelectedChannel(null);
    },
    onError: (error) => {
      toast.error(
        error instanceof Error ? error.message : "Failed to delete channel"
      );
    },
  });

  const toggleMutation = useMutation({
    mutationFn: ({
      channelId,
      enabled,
    }: {
      channelId: string;
      enabled: boolean;
    }) => api.updateTeamNotificationChannel(teamId, channelId, { enabled }),
    onSuccess: () => {
      toast.success("Channel updated");
      queryClient.invalidateQueries({
        queryKey: ["team-notification-channels", teamId],
      });
    },
    onError: (error) => {
      toast.error(
        error instanceof Error ? error.message : "Failed to update channel"
      );
    },
  });

  const testMutation = useMutation({
    mutationFn: (channelId: string) =>
      api.testTeamNotificationChannel(teamId, channelId),
    onSuccess: () => {
      toast.success("Test notification sent");
    },
    onError: (error) => {
      toast.error(
        error instanceof Error
          ? error.message
          : "Failed to send test notification"
      );
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
    setPayloadTemplate("json");
    setCustomTemplate("");
    setChannelType("slack");
  };

  const handleCreateSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!formName.trim()) {
      toast.error("Name is required");
      return;
    }

    if (
      channelType === "slack" ||
      channelType === "discord" ||
      channelType === "webhook"
    ) {
      if (!webhookUrl.trim()) {
        toast.error("URL is required");
        return;
      }
      if (channelType === "webhook" && !webhookUrl.startsWith("https://")) {
        toast.error("Webhook URL must use HTTPS");
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
      const addresses = toAddresses
        .split(",")
        .map((a) => a.trim())
        .filter((a) => a);
      if (addresses.length === 0) {
        toast.error("At least one recipient address is required");
        return;
      }
    }

    createMutation.mutate();
  };

  const isSubmitting =
    createMutation.isPending ||
    deleteMutation.isPending ||
    toggleMutation.isPending ||
    testMutation.isPending;

  return (
    <>
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Bell className="h-5 w-5" />
                Notification Channels
              </CardTitle>
              <CardDescription>
                Configure alert notifications for this team via Slack, Discord,
                Email, or Webhook.
              </CardDescription>
            </div>
            <Button onClick={() => setShowCreateDialog(true)}>
              <Plus className="h-4 w-4 mr-2" />
              Add Channel
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {channelsLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin" />
            </div>
          ) : channels.length === 0 ? (
            <p className="text-muted-foreground py-4 text-center">
              No notification channels configured. Add one to receive resource
              alerts.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead className="w-32">Actions</TableHead>
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
                          {channel.channel_type.charAt(0).toUpperCase() +
                            channel.channel_type.slice(1)}
                        </span>
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="p-0"
                        disabled={isSubmitting}
                        onClick={() =>
                          toggleMutation.mutate({
                            channelId: channel.id,
                            enabled: !channel.enabled,
                          })
                        }
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
                          disabled={isSubmitting || !channel.enabled}
                          onClick={() => testMutation.mutate(channel.id)}
                          title="Send test notification"
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
      <Dialog
        open={showCreateDialog}
        onOpenChange={(open) => {
          setShowCreateDialog(open);
          if (!open) resetCreateForm();
        }}
      >
        <DialogContent className="max-w-lg">
          <form onSubmit={handleCreateSubmit}>
            <DialogHeader>
              <DialogTitle>Add Notification Channel</DialogTitle>
              <DialogDescription>
                Configure a new channel to receive resource alert notifications
                for this team.
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
                <Select
                  value={channelType}
                  onValueChange={(v) =>
                    setChannelType(v as TeamNotificationChannelType)
                  }
                >
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
                    <SelectItem value="webhook">
                      <span className="flex items-center gap-2">
                        <Webhook className="h-4 w-4" />
                        Webhook
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

              {/* Webhook Config */}
              {channelType === "webhook" && (
                <>
                  <div className="space-y-2">
                    <Label htmlFor="webhook_url">Webhook URL (HTTPS)</Label>
                    <Input
                      id="webhook_url"
                      value={webhookUrl}
                      onChange={(e) => setWebhookUrl(e.target.value)}
                      placeholder="https://your-webhook-endpoint.com/..."
                      required
                    />
                  </div>
                  <div className="space-y-2">
                    <Label>Payload Template</Label>
                    <Select
                      value={payloadTemplate}
                      onValueChange={(v) =>
                        setPayloadTemplate(
                          v as "json" | "slack" | "discord" | "custom"
                        )
                      }
                    >
                      <SelectTrigger>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {PAYLOAD_TEMPLATES.map((template) => (
                          <SelectItem key={template.value} value={template.value}>
                            <div className="flex flex-col">
                              <span>{template.label}</span>
                              <span className="text-xs text-muted-foreground">
                                {template.description}
                              </span>
                            </div>
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                  {payloadTemplate === "custom" && (
                    <div className="space-y-2">
                      <Label htmlFor="custom_template">Custom Template</Label>
                      <Textarea
                        id="custom_template"
                        value={customTemplate}
                        onChange={(e) => setCustomTemplate(e.target.value)}
                        placeholder={`{"text": "Alert: {{app_name}} - {{metric_type}} at {{value}}%"}`}
                        rows={4}
                      />
                      <p className="text-xs text-muted-foreground">
                        Available variables: {"{{app_name}}"}, {"{{metric_type}}"},
                        {"{{value}}"}, {"{{threshold}}"}, {"{{severity}}"},
                        {"{{status}}"}, {"{{dashboard_url}}"}
                      </p>
                    </div>
                  )}
                </>
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
                    <Label htmlFor="to_addresses">
                      To Addresses (comma-separated)
                    </Label>
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
              Are you sure you want to delete "{selectedChannel?.name}"? This
              action cannot be undone.
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
              onClick={() =>
                selectedChannel && deleteMutation.mutate(selectedChannel.id)
              }
            >
              {deleteMutation.isPending ? "Deleting..." : "Delete"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
