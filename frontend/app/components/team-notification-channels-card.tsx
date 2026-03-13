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
  TelegramConfig,
  TeamsConfig,
  PushoverConfig,
  NtfyConfig,
  MattermostConfig,
  LarkConfig,
  GotifyConfig,
  ResendConfig,
} from "@/types/api";
import {
  Loader2,
  Plus,
  Trash2,
  Send,
  Bell,
  BellRing,
  MessageSquare,
  Mail,
  Webhook,
  Check,
  X,
  BotMessageSquare,
  Users,
} from "lucide-react";
import {
  SlackConfigFields,
  DiscordConfigFields,
  TelegramConfigFields,
  TeamsConfigFields,
  PushoverConfigFields,
  NtfyConfigFields,
  EmailConfigFields,
  WebhookConfigFields,
  MattermostConfigFields,
  LarkConfigFields,
  GotifyConfigFields,
  ResendConfigFields,
} from "@/components/notifications/channel-config-fields";

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
    case "telegram":
      return <BotMessageSquare className="h-4 w-4" />;
    case "teams":
      return <Users className="h-4 w-4" />;
    case "pushover":
      return <Bell className="h-4 w-4" />;
    case "ntfy":
      return <BellRing className="h-4 w-4" />;
    case "mattermost":
      return <MessageSquare className="h-4 w-4" />;
    case "lark":
      return <MessageSquare className="h-4 w-4" />;
    case "gotify":
      return <BellRing className="h-4 w-4" />;
    case "resend":
      return <Mail className="h-4 w-4" />;
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
    case "telegram":
      return "default";
    case "teams":
      return "secondary";
    case "pushover":
      return "default";
    case "ntfy":
      return "secondary";
    case "mattermost":
      return "default";
    case "lark":
      return "secondary";
    case "gotify":
      return "default";
    case "resend":
      return "outline";
  }
}

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
  const [botToken, setBotToken] = useState("");
  const [chatId, setChatId] = useState("");
  const [topicId, setTopicId] = useState("");
  const [teamsWebhookUrl, setTeamsWebhookUrl] = useState("");
  const [pushoverUserKey, setPushoverUserKey] = useState("");
  const [pushoverAppToken, setPushoverAppToken] = useState("");
  const [pushoverDevice, setPushoverDevice] = useState("");
  const [pushoverPriority, setPushoverPriority] = useState("0");
  const [ntfyTopic, setNtfyTopic] = useState("");
  const [ntfyServerUrl, setNtfyServerUrl] = useState("");
  const [ntfyPriority, setNtfyPriority] = useState("3");
  const [ntfyTags, setNtfyTags] = useState("");
  const [mattermostWebhookUrl, setMattermostWebhookUrl] = useState("");
  const [larkWebhookUrl, setLarkWebhookUrl] = useState("");
  const [gotifyServerUrl, setGotifyServerUrl] = useState("");
  const [gotifyAppToken, setGotifyAppToken] = useState("");
  const [gotifyPriority, setGotifyPriority] = useState("5");
  const [resendApiKey, setResendApiKey] = useState("");
  const [resendFromAddress, setResendFromAddress] = useState("");
  const [resendToAddresses, setResendToAddresses] = useState("");
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
      let config:
        | SlackConfig
        | DiscordConfig
        | EmailConfig
        | WebhookConfig
        | TelegramConfig
        | TeamsConfig
        | PushoverConfig
        | NtfyConfig
        | MattermostConfig
        | LarkConfig
        | GotifyConfig
        | ResendConfig;

      if (channelType === "slack") {
        config = { webhook_url: webhookUrl.trim() };
      } else if (channelType === "discord") {
        config = { webhook_url: webhookUrl.trim() };
      } else if (channelType === "telegram") {
        config = {
          bot_token: botToken.trim(),
          chat_id: chatId.trim(),
          topic_id: topicId.trim() ? parseInt(topicId.trim(), 10) : undefined,
        };
      } else if (channelType === "teams") {
        config = { webhook_url: teamsWebhookUrl.trim() };
      } else if (channelType === "pushover") {
        config = {
          user_key: pushoverUserKey.trim(),
          app_token: pushoverAppToken.trim(),
          device: pushoverDevice.trim() || undefined,
          priority: parseInt(pushoverPriority, 10),
        };
      } else if (channelType === "ntfy") {
        config = {
          topic: ntfyTopic.trim(),
          server_url: ntfyServerUrl.trim() || undefined,
          priority: parseInt(ntfyPriority, 10),
          tags: ntfyTags.trim() || undefined,
        };
      } else if (channelType === "webhook") {
        config = {
          url: webhookUrl.trim(),
          payload_template: payloadTemplate,
          custom_template: payloadTemplate === "custom" ? customTemplate : undefined,
        };
      } else if (channelType === "mattermost") {
        config = { webhook_url: mattermostWebhookUrl.trim() };
      } else if (channelType === "lark") {
        config = { webhook_url: larkWebhookUrl.trim() };
      } else if (channelType === "gotify") {
        config = {
          server_url: gotifyServerUrl.trim(),
          app_token: gotifyAppToken.trim(),
          priority: parseInt(gotifyPriority, 10),
        };
      } else if (channelType === "resend") {
        const addresses = resendToAddresses
          .split(",")
          .map((a) => a.trim())
          .filter((a) => a);
        config = {
          api_key: resendApiKey.trim(),
          from_address: resendFromAddress.trim(),
          to_addresses: addresses,
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
    setBotToken("");
    setChatId("");
    setTopicId("");
    setTeamsWebhookUrl("");
    setPushoverUserKey("");
    setPushoverAppToken("");
    setPushoverDevice("");
    setPushoverPriority("0");
    setNtfyTopic("");
    setNtfyServerUrl("");
    setNtfyPriority("3");
    setNtfyTags("");
    setMattermostWebhookUrl("");
    setLarkWebhookUrl("");
    setGotifyServerUrl("");
    setGotifyAppToken("");
    setGotifyPriority("5");
    setResendApiKey("");
    setResendFromAddress("");
    setResendToAddresses("");
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
    } else if (channelType === "telegram") {
      if (!botToken.trim()) {
        toast.error("Bot token is required");
        return;
      }
      if (!chatId.trim()) {
        toast.error("Chat ID is required");
        return;
      }
    } else if (channelType === "teams") {
      if (!teamsWebhookUrl.trim()) {
        toast.error("Webhook URL is required");
        return;
      }
      if (!teamsWebhookUrl.trim().startsWith("https://")) {
        toast.error("Webhook URL must use HTTPS");
        return;
      }
    } else if (channelType === "pushover") {
      if (!pushoverUserKey.trim()) {
        toast.error("User key is required");
        return;
      }
      if (!pushoverAppToken.trim()) {
        toast.error("App token is required");
        return;
      }
    } else if (channelType === "ntfy") {
      if (!ntfyTopic.trim()) {
        toast.error("Topic is required");
        return;
      }
    } else if (channelType === "mattermost") {
      if (!mattermostWebhookUrl.trim()) {
        toast.error("Webhook URL is required");
        return;
      }
    } else if (channelType === "lark") {
      if (!larkWebhookUrl.trim()) {
        toast.error("Webhook URL is required");
        return;
      }
    } else if (channelType === "gotify") {
      if (!gotifyServerUrl.trim()) {
        toast.error("Server URL is required");
        return;
      }
      if (!gotifyAppToken.trim()) {
        toast.error("App token is required");
        return;
      }
    } else if (channelType === "resend") {
      if (!resendApiKey.trim()) {
        toast.error("API key is required");
        return;
      }
      if (!resendFromAddress.trim()) {
        toast.error("From address is required");
        return;
      }
      const addresses = resendToAddresses.split(",").map((a) => a.trim()).filter((a) => a);
      if (addresses.length === 0) {
        toast.error("At least one recipient address is required");
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
                Configure alert notifications for this team via Slack, Discord, Email, Webhook,
                Telegram, Microsoft Teams, Pushover, ntfy, Mattermost, Lark, Gotify, or Resend.
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
              No notification channels configured. Add one to receive resource alerts.
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
                Configure a new channel to receive resource alert notifications for this team.
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
                  onValueChange={(v) => setChannelType(v as TeamNotificationChannelType)}
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
                    <SelectItem value="telegram">
                      <span className="flex items-center gap-2">
                        <BotMessageSquare className="h-4 w-4" />
                        Telegram
                      </span>
                    </SelectItem>
                    <SelectItem value="teams">
                      <span className="flex items-center gap-2">
                        <Users className="h-4 w-4" />
                        Microsoft Teams
                      </span>
                    </SelectItem>
                    <SelectItem value="pushover">
                      <span className="flex items-center gap-2">
                        <Bell className="h-4 w-4" />
                        Pushover
                      </span>
                    </SelectItem>
                    <SelectItem value="ntfy">
                      <span className="flex items-center gap-2">
                        <BellRing className="h-4 w-4" />
                        ntfy
                      </span>
                    </SelectItem>
                    <SelectItem value="mattermost">
                      <span className="flex items-center gap-2">
                        <MessageSquare className="h-4 w-4" />
                        Mattermost
                      </span>
                    </SelectItem>
                    <SelectItem value="lark">
                      <span className="flex items-center gap-2">
                        <MessageSquare className="h-4 w-4" />
                        Lark (Feishu)
                      </span>
                    </SelectItem>
                    <SelectItem value="gotify">
                      <span className="flex items-center gap-2">
                        <BellRing className="h-4 w-4" />
                        Gotify
                      </span>
                    </SelectItem>
                    <SelectItem value="resend">
                      <span className="flex items-center gap-2">
                        <Mail className="h-4 w-4" />
                        Resend (Email API)
                      </span>
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {channelType === "slack" && (
                <SlackConfigFields webhookUrl={webhookUrl} setWebhookUrl={setWebhookUrl} />
              )}
              {channelType === "discord" && (
                <DiscordConfigFields webhookUrl={webhookUrl} setWebhookUrl={setWebhookUrl} />
              )}
              {channelType === "webhook" && (
                <WebhookConfigFields
                  webhookUrl={webhookUrl}
                  setWebhookUrl={setWebhookUrl}
                  payloadTemplate={payloadTemplate}
                  setPayloadTemplate={setPayloadTemplate}
                  customTemplate={customTemplate}
                  setCustomTemplate={setCustomTemplate}
                />
              )}
              {channelType === "telegram" && (
                <TelegramConfigFields
                  botToken={botToken}
                  setBotToken={setBotToken}
                  chatId={chatId}
                  setChatId={setChatId}
                  topicId={topicId}
                  setTopicId={setTopicId}
                />
              )}
              {channelType === "teams" && (
                <TeamsConfigFields
                  teamsWebhookUrl={teamsWebhookUrl}
                  setTeamsWebhookUrl={setTeamsWebhookUrl}
                />
              )}
              {channelType === "pushover" && (
                <PushoverConfigFields
                  pushoverUserKey={pushoverUserKey}
                  setPushoverUserKey={setPushoverUserKey}
                  pushoverAppToken={pushoverAppToken}
                  setPushoverAppToken={setPushoverAppToken}
                  pushoverDevice={pushoverDevice}
                  setPushoverDevice={setPushoverDevice}
                  pushoverPriority={pushoverPriority}
                  setPushoverPriority={setPushoverPriority}
                />
              )}
              {channelType === "ntfy" && (
                <NtfyConfigFields
                  ntfyTopic={ntfyTopic}
                  setNtfyTopic={setNtfyTopic}
                  ntfyServerUrl={ntfyServerUrl}
                  setNtfyServerUrl={setNtfyServerUrl}
                  ntfyPriority={ntfyPriority}
                  setNtfyPriority={setNtfyPriority}
                  ntfyTags={ntfyTags}
                  setNtfyTags={setNtfyTags}
                />
              )}
              {channelType === "email" && (
                <EmailConfigFields
                  smtpHost={smtpHost}
                  setSmtpHost={setSmtpHost}
                  smtpPort={smtpPort}
                  setSmtpPort={setSmtpPort}
                  smtpUsername={smtpUsername}
                  setSmtpUsername={setSmtpUsername}
                  smtpPassword={smtpPassword}
                  setSmtpPassword={setSmtpPassword}
                  smtpTls={smtpTls}
                  setSmtpTls={setSmtpTls}
                  fromAddress={fromAddress}
                  setFromAddress={setFromAddress}
                  toAddresses={toAddresses}
                  setToAddresses={setToAddresses}
                />
              )}
              {channelType === "mattermost" && (
                <MattermostConfigFields
                  webhookUrl={mattermostWebhookUrl}
                  setWebhookUrl={setMattermostWebhookUrl}
                />
              )}
              {channelType === "lark" && (
                <LarkConfigFields
                  webhookUrl={larkWebhookUrl}
                  setWebhookUrl={setLarkWebhookUrl}
                />
              )}
              {channelType === "gotify" && (
                <GotifyConfigFields
                  gotifyServerUrl={gotifyServerUrl}
                  setGotifyServerUrl={setGotifyServerUrl}
                  gotifyAppToken={gotifyAppToken}
                  setGotifyAppToken={setGotifyAppToken}
                  gotifyPriority={gotifyPriority}
                  setGotifyPriority={setGotifyPriority}
                />
              )}
              {channelType === "resend" && (
                <ResendConfigFields
                  resendApiKey={resendApiKey}
                  setResendApiKey={setResendApiKey}
                  resendFromAddress={resendFromAddress}
                  setResendFromAddress={setResendFromAddress}
                  resendToAddresses={resendToAddresses}
                  setResendToAddresses={setResendToAddresses}
                />
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
              Are you sure you want to delete "{selectedChannel?.name}"? This action cannot be
              undone.
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
    </>
  );
}
