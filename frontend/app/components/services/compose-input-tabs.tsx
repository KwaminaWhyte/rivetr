/**
 * Compose input with two modes: write the YAML yourself, or describe the
 * stack and let the configured AI provider draft it. The AI result is only a
 * preview: nothing is created until the user clicks "Use this compose" and
 * then submits the surrounding form.
 */
import { useState } from "react";
import { Link } from "react-router";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Sparkles, Loader2 } from "lucide-react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { aiApi, type GeneratedCompose } from "@/lib/api/ai";

/** Where the AI provider is configured (instance settings). */
export const AI_SETTINGS_PATH = "/settings";

interface ComposeInputTabsProps {
  value: string;
  onChange: (value: string) => void;
  /** id for the textarea, so an outer <Label htmlFor> keeps working. */
  textareaId?: string;
  rows?: number;
}

export function ComposeInputTabs({
  value,
  onChange,
  textareaId = "compose-content",
  rows = 12,
}: ComposeInputTabsProps) {
  const [tab, setTab] = useState<"write" | "describe">("write");

  return (
    <Tabs value={tab} onValueChange={(v) => setTab(v as "write" | "describe")}>
      <TabsList>
        <TabsTrigger value="write">Write compose</TabsTrigger>
        <TabsTrigger value="describe">
          <Sparkles className="mr-1 h-3.5 w-3.5" />
          Describe it
        </TabsTrigger>
      </TabsList>
      <TabsContent value="write" className="space-y-2">
        <Label htmlFor={textareaId}>Docker Compose Content</Label>
        <Textarea
          id={textareaId}
          name="compose_content"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder="Paste your docker-compose.yml content..."
          className="font-mono text-sm"
          rows={rows}
          required={tab === "write"}
        />
      </TabsContent>
      <TabsContent value="describe">
        <DescribeCompose
          currentCompose={value}
          onUse={(compose) => {
            onChange(compose);
            setTab("write");
          }}
        />
      </TabsContent>
    </Tabs>
  );
}

function DescribeCompose({
  currentCompose,
  onUse,
}: {
  currentCompose: string;
  onUse: (compose: string) => void;
}) {
  const [prompt, setPrompt] = useState("");
  const [modifyExisting, setModifyExisting] = useState(false);
  const [result, setResult] = useState<GeneratedCompose | null>(null);
  const [draft, setDraft] = useState("");

  const status = useQuery({
    queryKey: ["ai-status"],
    queryFn: aiApi.getStatus,
    staleTime: 60_000,
  });

  const generate = useMutation({
    mutationFn: () =>
      aiApi.generateCompose(prompt.trim(), modifyExisting ? currentCompose : undefined),
    onSuccess: (data) => {
      setResult(data);
      setDraft(data.compose);
    },
  });

  if (status.isLoading) {
    return <p className="text-sm text-muted-foreground">Checking AI provider...</p>;
  }

  if (!status.data?.configured) {
    return (
      <Alert>
        <Sparkles className="h-4 w-4" />
        <AlertTitle>No AI provider configured</AlertTitle>
        <AlertDescription>
          <p>
            Describing a stack in plain words needs an AI provider (Claude, OpenAI, Gemini or
            Moonshot). An admin can add one in{" "}
            <Link to={AI_SETTINGS_PATH} className="underline underline-offset-2">
              Settings, AI provider
            </Link>
            . You can still paste a compose file in the Write compose tab.
          </p>
        </AlertDescription>
      </Alert>
    );
  }

  const hasExisting = currentCompose.trim().length > 0;

  return (
    <div className="space-y-3">
      <div className="space-y-2">
        <Label htmlFor="compose-prompt">What do you want to run?</Label>
        <Textarea
          id="compose-prompt"
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          placeholder="A WordPress site with MariaDB and Redis object cache"
          rows={3}
          maxLength={4000}
        />
      </div>
      {hasExisting && (
        <label className="flex items-center gap-2 text-sm text-muted-foreground">
          <input
            type="checkbox"
            checked={modifyExisting}
            onChange={(e) => setModifyExisting(e.target.checked)}
          />
          Modify the compose in the Write compose tab instead of starting fresh
        </label>
      )}
      <div className="flex items-center gap-2">
        <Button
          type="button"
          variant="secondary"
          disabled={!prompt.trim() || generate.isPending}
          onClick={() => generate.mutate()}
        >
          {generate.isPending ? (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          ) : (
            <Sparkles className="mr-2 h-4 w-4" />
          )}
          {result ? "Regenerate" : "Generate compose"}
        </Button>
        <span className="text-xs text-muted-foreground">
          Nothing is deployed. Review the result first.
        </span>
      </div>
      {generate.isError && (
        <p className="text-sm text-destructive">
          {generate.error instanceof Error ? generate.error.message : "Generation failed"}
        </p>
      )}
      {result && (
        <div className="space-y-2">
          {result.explanation && (
            <p className="text-sm text-muted-foreground">{result.explanation}</p>
          )}
          <div className="flex flex-wrap gap-1">
            {result.services.map((s) => (
              <Badge key={s} variant="outline">
                {s}
              </Badge>
            ))}
          </div>
          <Label htmlFor="compose-ai-preview">Generated compose (editable)</Label>
          <Textarea
            id="compose-ai-preview"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            className="font-mono text-sm"
            rows={14}
          />
          <Button type="button" disabled={!draft.trim()} onClick={() => onUse(draft)}>
            Use this compose
          </Button>
        </div>
      )}
    </div>
  );
}
