/**
 * Reusable form field blocks for each notification channel type.
 * Used by both the personal notifications page and team notification channels card.
 */
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

// ---- Slack ----

interface SlackFieldsProps {
  webhookUrl: string;
  setWebhookUrl: (v: string) => void;
}

export function SlackConfigFields({ webhookUrl, setWebhookUrl }: SlackFieldsProps) {
  return (
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
  );
}

// ---- Discord ----

interface DiscordFieldsProps {
  webhookUrl: string;
  setWebhookUrl: (v: string) => void;
}

export function DiscordConfigFields({ webhookUrl, setWebhookUrl }: DiscordFieldsProps) {
  return (
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
  );
}

// ---- Telegram ----

interface TelegramFieldsProps {
  botToken: string;
  setBotToken: (v: string) => void;
  chatId: string;
  setChatId: (v: string) => void;
  topicId: string;
  setTopicId: (v: string) => void;
}

export function TelegramConfigFields({
  botToken,
  setBotToken,
  chatId,
  setChatId,
  topicId,
  setTopicId,
}: TelegramFieldsProps) {
  return (
    <>
      <div className="space-y-2">
        <Label htmlFor="bot_token">Bot Token</Label>
        <Input
          id="bot_token"
          type="password"
          value={botToken}
          onChange={(e) => setBotToken(e.target.value)}
          placeholder="123456:ABC-DEF..."
          required
        />
        <p className="text-xs text-muted-foreground">
          Get this from @BotFather on Telegram.
        </p>
      </div>
      <div className="space-y-2">
        <Label htmlFor="chat_id">Chat ID</Label>
        <Input
          id="chat_id"
          value={chatId}
          onChange={(e) => setChatId(e.target.value)}
          placeholder="-1001234567890"
          required
        />
        <p className="text-xs text-muted-foreground">
          The chat, group, or channel ID. Use @userinfobot to find yours.
        </p>
      </div>
      <div className="space-y-2">
        <Label htmlFor="topic_id">Topic ID (optional)</Label>
        <Input
          id="topic_id"
          type="number"
          value={topicId}
          onChange={(e) => setTopicId(e.target.value)}
          placeholder="e.g., 123"
        />
        <p className="text-xs text-muted-foreground">
          For forum/topic groups, specify the topic thread ID.
        </p>
      </div>
    </>
  );
}

// ---- Microsoft Teams ----

interface TeamsFieldsProps {
  teamsWebhookUrl: string;
  setTeamsWebhookUrl: (v: string) => void;
}

export function TeamsConfigFields({ teamsWebhookUrl, setTeamsWebhookUrl }: TeamsFieldsProps) {
  return (
    <div className="space-y-2">
      <Label htmlFor="teams_webhook_url">Webhook URL</Label>
      <Input
        id="teams_webhook_url"
        value={teamsWebhookUrl}
        onChange={(e) => setTeamsWebhookUrl(e.target.value)}
        placeholder="https://outlook.office.com/webhook/..."
        required
      />
      <p className="text-xs text-muted-foreground">
        Create an Incoming Webhook connector in your Teams channel settings.
      </p>
    </div>
  );
}

// ---- Pushover ----

interface PushoverFieldsProps {
  pushoverUserKey: string;
  setPushoverUserKey: (v: string) => void;
  pushoverAppToken: string;
  setPushoverAppToken: (v: string) => void;
  pushoverDevice: string;
  setPushoverDevice: (v: string) => void;
  pushoverPriority: string;
  setPushoverPriority: (v: string) => void;
}

export function PushoverConfigFields({
  pushoverUserKey,
  setPushoverUserKey,
  pushoverAppToken,
  setPushoverAppToken,
  pushoverDevice,
  setPushoverDevice,
  pushoverPriority,
  setPushoverPriority,
}: PushoverFieldsProps) {
  return (
    <>
      <div className="space-y-2">
        <Label htmlFor="pushover_user_key">User Key</Label>
        <Input
          id="pushover_user_key"
          value={pushoverUserKey}
          onChange={(e) => setPushoverUserKey(e.target.value)}
          placeholder="Your Pushover user key"
          required
        />
        <p className="text-xs text-muted-foreground">
          Find this in your Pushover dashboard settings.
        </p>
      </div>
      <div className="space-y-2">
        <Label htmlFor="pushover_app_token">App Token</Label>
        <Input
          id="pushover_app_token"
          type="password"
          value={pushoverAppToken}
          onChange={(e) => setPushoverAppToken(e.target.value)}
          placeholder="Your Pushover application API token"
          required
        />
        <p className="text-xs text-muted-foreground">
          Create an application at pushover.net to get an API token.
        </p>
      </div>
      <div className="space-y-2">
        <Label htmlFor="pushover_device">Device (optional)</Label>
        <Input
          id="pushover_device"
          value={pushoverDevice}
          onChange={(e) => setPushoverDevice(e.target.value)}
          placeholder="e.g., iphone, desktop"
        />
        <p className="text-xs text-muted-foreground">
          Send to a specific device instead of all devices.
        </p>
      </div>
      <div className="space-y-2">
        <Label htmlFor="pushover_priority">Priority</Label>
        <Select value={pushoverPriority} onValueChange={setPushoverPriority}>
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="-2">-2 (Silent)</SelectItem>
            <SelectItem value="-1">-1 (Quiet)</SelectItem>
            <SelectItem value="0">0 (Normal)</SelectItem>
            <SelectItem value="1">1 (High)</SelectItem>
            <SelectItem value="2">2 (Emergency)</SelectItem>
          </SelectContent>
        </Select>
      </div>
    </>
  );
}

// ---- Ntfy ----

interface NtfyFieldsProps {
  ntfyTopic: string;
  setNtfyTopic: (v: string) => void;
  ntfyServerUrl: string;
  setNtfyServerUrl: (v: string) => void;
  ntfyPriority: string;
  setNtfyPriority: (v: string) => void;
  ntfyTags: string;
  setNtfyTags: (v: string) => void;
}

export function NtfyConfigFields({
  ntfyTopic,
  setNtfyTopic,
  ntfyServerUrl,
  setNtfyServerUrl,
  ntfyPriority,
  setNtfyPriority,
  ntfyTags,
  setNtfyTags,
}: NtfyFieldsProps) {
  return (
    <>
      <div className="space-y-2">
        <Label htmlFor="ntfy_topic">Topic</Label>
        <Input
          id="ntfy_topic"
          value={ntfyTopic}
          onChange={(e) => setNtfyTopic(e.target.value)}
          placeholder="rivetr-alerts"
          required
        />
        <p className="text-xs text-muted-foreground">
          The ntfy topic to publish to. Choose a unique, hard-to-guess name.
        </p>
      </div>
      <div className="space-y-2">
        <Label htmlFor="ntfy_server_url">Server URL (optional)</Label>
        <Input
          id="ntfy_server_url"
          value={ntfyServerUrl}
          onChange={(e) => setNtfyServerUrl(e.target.value)}
          placeholder="https://ntfy.sh"
        />
        <p className="text-xs text-muted-foreground">
          Leave empty to use the default ntfy.sh server, or enter your self-hosted instance URL.
        </p>
      </div>
      <div className="space-y-2">
        <Label htmlFor="ntfy_priority">Priority</Label>
        <Select value={ntfyPriority} onValueChange={setNtfyPriority}>
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="1">1 (Min)</SelectItem>
            <SelectItem value="2">2 (Low)</SelectItem>
            <SelectItem value="3">3 (Default)</SelectItem>
            <SelectItem value="4">4 (High)</SelectItem>
            <SelectItem value="5">5 (Max/Urgent)</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div className="space-y-2">
        <Label htmlFor="ntfy_tags">Tags (optional)</Label>
        <Input
          id="ntfy_tags"
          value={ntfyTags}
          onChange={(e) => setNtfyTags(e.target.value)}
          placeholder="warning,server"
        />
        <p className="text-xs text-muted-foreground">
          Comma-separated tags/emojis for the notification (e.g., warning,server).
        </p>
      </div>
    </>
  );
}

// ---- Email (SMTP) ----

interface EmailFieldsProps {
  smtpHost: string;
  setSmtpHost: (v: string) => void;
  smtpPort: string;
  setSmtpPort: (v: string) => void;
  smtpUsername: string;
  setSmtpUsername: (v: string) => void;
  smtpPassword: string;
  setSmtpPassword: (v: string) => void;
  smtpTls: boolean;
  setSmtpTls: (v: boolean) => void;
  fromAddress: string;
  setFromAddress: (v: string) => void;
  toAddresses: string;
  setToAddresses: (v: string) => void;
}

export function EmailConfigFields({
  smtpHost,
  setSmtpHost,
  smtpPort,
  setSmtpPort,
  smtpUsername,
  setSmtpUsername,
  smtpPassword,
  setSmtpPassword,
  smtpTls,
  setSmtpTls,
  fromAddress,
  setFromAddress,
  toAddresses,
  setToAddresses,
}: EmailFieldsProps) {
  return (
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
        <Switch id="smtp_tls" checked={smtpTls} onCheckedChange={setSmtpTls} />
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
  );
}

// ---- Mattermost ----

interface MattermostFieldsProps {
  webhookUrl: string;
  setWebhookUrl: (v: string) => void;
}

export function MattermostConfigFields({ webhookUrl, setWebhookUrl }: MattermostFieldsProps) {
  return (
    <div className="space-y-2">
      <Label htmlFor="mattermost_webhook_url">Webhook URL</Label>
      <Input
        id="mattermost_webhook_url"
        value={webhookUrl}
        onChange={(e) => setWebhookUrl(e.target.value)}
        placeholder="https://your-mattermost.com/hooks/..."
        required
      />
      <p className="text-xs text-muted-foreground">
        Create an Incoming Webhook integration in your Mattermost channel settings.
      </p>
    </div>
  );
}

// ---- Lark (Feishu) ----

interface LarkFieldsProps {
  webhookUrl: string;
  setWebhookUrl: (v: string) => void;
}

export function LarkConfigFields({ webhookUrl, setWebhookUrl }: LarkFieldsProps) {
  return (
    <div className="space-y-2">
      <Label htmlFor="lark_webhook_url">Webhook URL</Label>
      <Input
        id="lark_webhook_url"
        value={webhookUrl}
        onChange={(e) => setWebhookUrl(e.target.value)}
        placeholder="https://open.larksuite.com/open-apis/bot/v2/hook/..."
        required
      />
      <p className="text-xs text-muted-foreground">
        Get this from your Lark/Feishu group bot configuration.
      </p>
    </div>
  );
}

// ---- Gotify ----

interface GotifyFieldsProps {
  gotifyServerUrl: string;
  setGotifyServerUrl: (v: string) => void;
  gotifyAppToken: string;
  setGotifyAppToken: (v: string) => void;
  gotifyPriority: string;
  setGotifyPriority: (v: string) => void;
}

export function GotifyConfigFields({
  gotifyServerUrl,
  setGotifyServerUrl,
  gotifyAppToken,
  setGotifyAppToken,
  gotifyPriority,
  setGotifyPriority,
}: GotifyFieldsProps) {
  return (
    <>
      <div className="space-y-2">
        <Label htmlFor="gotify_server_url">Server URL</Label>
        <Input
          id="gotify_server_url"
          value={gotifyServerUrl}
          onChange={(e) => setGotifyServerUrl(e.target.value)}
          placeholder="https://gotify.example.com"
          required
        />
        <p className="text-xs text-muted-foreground">
          The base URL of your Gotify server.
        </p>
      </div>
      <div className="space-y-2">
        <Label htmlFor="gotify_app_token">App Token</Label>
        <Input
          id="gotify_app_token"
          type="password"
          value={gotifyAppToken}
          onChange={(e) => setGotifyAppToken(e.target.value)}
          placeholder="Your Gotify application token"
          required
        />
        <p className="text-xs text-muted-foreground">
          Create an application in your Gotify dashboard to get a token.
        </p>
      </div>
      <div className="space-y-2">
        <Label htmlFor="gotify_priority">Priority</Label>
        <Select value={gotifyPriority} onValueChange={setGotifyPriority}>
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="1">1 (Min)</SelectItem>
            <SelectItem value="3">3 (Low)</SelectItem>
            <SelectItem value="5">5 (Normal)</SelectItem>
            <SelectItem value="7">7 (High)</SelectItem>
            <SelectItem value="10">10 (Max/Urgent)</SelectItem>
          </SelectContent>
        </Select>
      </div>
    </>
  );
}

// ---- Resend ----

interface ResendFieldsProps {
  resendApiKey: string;
  setResendApiKey: (v: string) => void;
  resendFromAddress: string;
  setResendFromAddress: (v: string) => void;
  resendToAddresses: string;
  setResendToAddresses: (v: string) => void;
}

export function ResendConfigFields({
  resendApiKey,
  setResendApiKey,
  resendFromAddress,
  setResendFromAddress,
  resendToAddresses,
  setResendToAddresses,
}: ResendFieldsProps) {
  return (
    <>
      <div className="space-y-2">
        <Label htmlFor="resend_api_key">API Key</Label>
        <Input
          id="resend_api_key"
          type="password"
          value={resendApiKey}
          onChange={(e) => setResendApiKey(e.target.value)}
          placeholder="re_..."
          required
        />
        <p className="text-xs text-muted-foreground">
          Get your API key from the Resend dashboard at resend.com.
        </p>
      </div>
      <div className="space-y-2">
        <Label htmlFor="resend_from_address">From Address</Label>
        <Input
          id="resend_from_address"
          value={resendFromAddress}
          onChange={(e) => setResendFromAddress(e.target.value)}
          placeholder="noreply@yourdomain.com"
          required
        />
        <p className="text-xs text-muted-foreground">
          Must be a verified sender domain in your Resend account.
        </p>
      </div>
      <div className="space-y-2">
        <Label htmlFor="resend_to_addresses">To Addresses (comma-separated)</Label>
        <Input
          id="resend_to_addresses"
          value={resendToAddresses}
          onChange={(e) => setResendToAddresses(e.target.value)}
          placeholder="admin@example.com, devops@example.com"
          required
        />
      </div>
    </>
  );
}

// ---- Webhook (team channels only) ----

const PAYLOAD_TEMPLATES: {
  value: "json" | "slack" | "discord" | "custom";
  label: string;
  description: string;
}[] = [
  { value: "json", label: "JSON", description: "Standard JSON payload" },
  { value: "slack", label: "Slack", description: "Slack webhook format" },
  { value: "discord", label: "Discord", description: "Discord webhook format" },
  { value: "custom", label: "Custom", description: "Custom template with variables" },
];

interface WebhookFieldsProps {
  webhookUrl: string;
  setWebhookUrl: (v: string) => void;
  payloadTemplate: "json" | "slack" | "discord" | "custom";
  setPayloadTemplate: (v: "json" | "slack" | "discord" | "custom") => void;
  customTemplate: string;
  setCustomTemplate: (v: string) => void;
}

export function WebhookConfigFields({
  webhookUrl,
  setWebhookUrl,
  payloadTemplate,
  setPayloadTemplate,
  customTemplate,
  setCustomTemplate,
}: WebhookFieldsProps) {
  return (
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
          onValueChange={(v) => setPayloadTemplate(v as "json" | "slack" | "discord" | "custom")}
        >
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {PAYLOAD_TEMPLATES.map((template) => (
              <SelectItem key={template.value} value={template.value}>
                <div className="flex flex-col">
                  <span>{template.label}</span>
                  <span className="text-xs text-muted-foreground">{template.description}</span>
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
            Available variables: {"{{app_name}}"}, {"{{metric_type}}"}, {"{{value}}"},
            {"{{threshold}}"}, {"{{severity}}"}, {"{{status}}"}, {"{{dashboard_url}}"}
          </p>
        </div>
      )}
    </>
  );
}
