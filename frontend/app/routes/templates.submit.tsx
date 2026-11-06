import { useState } from "react";
import { Link } from "react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { ArrowLeft, FileCode2, Loader2, Plus, Send, Trash2 } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { EmptyState, PageHeader } from "@/components/page";
import { QueryError } from "@/components/query-error";
import { templateSubmissionsApi } from "@/lib/api";
import type { SubmissionInput, TemplateSubmission } from "@/lib/api";
import { TEMPLATE_CATEGORIES } from "@/types/api";

export function meta() {
  return [
    { title: "Submit a template - Rivetr" },
    { name: "description", content: "Share a Docker Compose template with the community" },
  ];
}

const STATUS_STYLE: Record<string, string> = {
  pending: "bg-amber-500/15 text-amber-700 dark:text-amber-400",
  approved: "bg-green-500/15 text-green-700 dark:text-green-400",
  rejected: "bg-destructive/15 text-destructive",
};

const COMPOSE_PLACEHOLDER = `services:
  app:
    image: example/app:latest
    ports:
      - "8080:8080"
    environment:
      API_KEY: \${API_KEY}
    volumes:
      - app_data:/data

volumes:
  app_data:`;

function MySubmissions() {
  const query = useQuery({
    queryKey: ["template-submissions", "mine"],
    queryFn: () => templateSubmissionsApi.mine(),
  });
  const queryClient = useQueryClient();
  const remove = useMutation({
    mutationFn: (id: string) => templateSubmissionsApi.remove(id),
    onSuccess: () => {
      toast.success("Submission withdrawn");
      void queryClient.invalidateQueries({ queryKey: ["template-submissions"] });
    },
    onError: (e: Error) => toast.error(e.message || "Could not withdraw the submission"),
  });

  if (query.isLoading) {
    return (
      <div className="flex justify-center py-8">
        <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
      </div>
    );
  }
  if (query.error) {
    return <QueryError what="your submissions" error={query.error} onRetry={() => query.refetch()} />;
  }
  const items = query.data ?? [];
  if (items.length === 0) {
    return (
      <EmptyState
        icon={<FileCode2 />}
        title="No submissions yet"
        description="Templates you submit appear here with their review status."
        compact
        bordered
      />
    );
  }
  return (
    <ul className="divide-y rounded-lg border bg-card">
      {items.map((s: TemplateSubmission) => (
        <li key={s.id} className="flex flex-wrap items-start justify-between gap-3 p-4">
          <div className="min-w-0 space-y-1">
            <div className="flex flex-wrap items-center gap-2">
              <span className="font-medium">{s.name}</span>
              <Badge className={`border-0 ${STATUS_STYLE[s.status] ?? ""}`}>{s.status}</Badge>
            </div>
            <p className="truncate text-sm text-muted-foreground">{s.description}</p>
            {s.status === "rejected" && s.rejection_reason && (
              <p className="text-sm text-destructive">Rejected: {s.rejection_reason}</p>
            )}
            {s.status === "approved" && s.approved_template_id && (
              <p className="text-sm text-muted-foreground">
                Live in the catalogue.{" "}
                <Link to="/templates" className="underline underline-offset-2">
                  Browse templates
                </Link>
              </p>
            )}
          </div>
          {s.status === "pending" && (
            <Button
              size="sm"
              variant="outline"
              disabled={remove.isPending}
              onClick={() => remove.mutate(s.id)}
            >
              <Trash2 className="h-3.5 w-3.5" />
              Withdraw
            </Button>
          )}
        </li>
      ))}
    </ul>
  );
}

export default function SubmitTemplatePage() {
  const queryClient = useQueryClient();
  const [form, setForm] = useState<SubmissionInput>({
    name: "",
    description: "",
    category: TEMPLATE_CATEGORIES[0].id as SubmissionInput["category"],
    icon: "",
    compose_content: "",
    env_hints: [],
  });
  const [envRows, setEnvRows] = useState<
    { name: string; label: string; required: boolean; secret: boolean; default: string }[]
  >([]);

  const submit = useMutation({
    mutationFn: (data: SubmissionInput) => templateSubmissionsApi.submit(data),
    onSuccess: () => {
      toast.success("Submitted for review");
      setForm({
        name: "",
        description: "",
        category: TEMPLATE_CATEGORIES[0].id as SubmissionInput["category"],
        icon: "",
        compose_content: "",
        env_hints: [],
      });
      setEnvRows([]);
      void queryClient.invalidateQueries({ queryKey: ["template-submissions"] });
    },
    onError: (e: Error) => toast.error(e.message || "Could not submit the template"),
  });

  const setField = <K extends keyof SubmissionInput>(k: K, v: SubmissionInput[K]) =>
    setForm((f) => ({ ...f, [k]: v }));

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<FileCode2 />}
        title="Submit a template"
        description="Share a Docker Compose setup with the community. An instance admin reviews every submission before it joins the catalogue."
      />
      <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_380px]">
        <Card>
          <CardHeader>
            <CardTitle>Template</CardTitle>
            <CardDescription>
              Use ${"{VAR}"} for values the person deploying it should set. They are filled from the
              environment variables below or when the template is deployed.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="name">Name</Label>
                <Input
                  id="name"
                  value={form.name}
                  onChange={(e) => setField("name", e.target.value)}
                  placeholder="Uptime Kuma"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="category">Category</Label>
                <Select
                  value={form.category}
                  onValueChange={(v) => setField("category", v as SubmissionInput["category"])}
                >
                  <SelectTrigger id="category" aria-label="Category">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {TEMPLATE_CATEGORIES.map((c) => (
                      <SelectItem key={c.id} value={c.id}>
                        {c.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="description">Description</Label>
              <Input
                id="description"
                value={form.description}
                onChange={(e) => setField("description", e.target.value)}
                placeholder="Self-hosted monitoring with status pages"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="icon">Icon URL (optional)</Label>
              <Input
                id="icon"
                value={form.icon ?? ""}
                onChange={(e) => setField("icon", e.target.value)}
                placeholder="https://example.com/icon.svg"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="compose">Docker Compose file</Label>
              <Textarea
                id="compose"
                value={form.compose_content}
                onChange={(e) => setField("compose_content", e.target.value)}
                placeholder={COMPOSE_PLACEHOLDER}
                className="min-h-[260px] font-mono text-xs"
                spellCheck={false}
              />
            </div>
          </CardContent>
        </Card>

        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Environment variables</CardTitle>
              <CardDescription>Shown to whoever deploys the template.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              {envRows.map((row, i) => (
                <div key={i} className="space-y-2 rounded-md border p-3">
                  <div className="grid grid-cols-2 gap-2">
                    <Input
                      value={row.name}
                      onChange={(e) =>
                        setEnvRows((r) => r.map((x, j) => (j === i ? { ...x, name: e.target.value } : x)))
                      }
                      placeholder="API_KEY"
                      aria-label="Variable name"
                      className="font-mono text-xs"
                    />
                    <Input
                      value={row.label}
                      onChange={(e) =>
                        setEnvRows((r) => r.map((x, j) => (j === i ? { ...x, label: e.target.value } : x)))
                      }
                      placeholder="API key (optional)"
                      aria-label="Variable label"
                    />
                  </div>
                  <Input
                    value={row.default}
                    onChange={(e) =>
                      setEnvRows((r) => r.map((x, j) => (j === i ? { ...x, default: e.target.value } : x)))
                    }
                    placeholder="Default value (optional)"
                    aria-label="Default value"
                  />
                  <div className="flex items-center justify-between text-sm text-muted-foreground">
                    <div className="flex gap-3">
                      <label className="flex items-center gap-1.5">
                        <input
                          type="checkbox"
                          checked={row.required}
                          onChange={(e) =>
                            setEnvRows((r) =>
                              r.map((x, j) => (j === i ? { ...x, required: e.target.checked } : x))
                            )
                          }
                        />
                        Required
                      </label>
                      <label className="flex items-center gap-1.5">
                        <input
                          type="checkbox"
                          checked={row.secret}
                          onChange={(e) =>
                            setEnvRows((r) =>
                              r.map((x, j) => (j === i ? { ...x, secret: e.target.checked } : x))
                            )
                          }
                        />
                        Secret
                      </label>
                    </div>
                    <Button
                      size="sm"
                      variant="ghost"
                      aria-label="Remove variable"
                      onClick={() => setEnvRows((r) => r.filter((_, j) => j !== i))}
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                </div>
              ))}
              <Button
                variant="outline"
                size="sm"
                onClick={() =>
                  setEnvRows((r) => [...r, { name: "", label: "", required: false, secret: false, default: "" }])
                }
              >
                <Plus className="h-3.5 w-3.5" />
                Add variable
              </Button>
            </CardContent>
          </Card>

          <Button
            className="w-full"
            disabled={submit.isPending || !form.name.trim() || !form.compose_content.trim()}
            onClick={() =>
              submit.mutate({
                ...form,
                icon: form.icon?.trim() ? form.icon : null,
                env_hints: envRows
                  .filter((r) => r.name.trim())
                  .map((r) => ({
                    name: r.name.trim(),
                    label: r.label.trim() || r.name.trim(),
                    required: r.required,
                    secret: r.secret,
                    default: r.default,
                  })),
              })
            }
          >
            {submit.isPending ? <Loader2 className="h-4 w-4 animate-spin" /> : <Send className="h-4 w-4" />}
            Submit for review
          </Button>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Your submissions</CardTitle>
          <CardDescription>Pending ones can be withdrawn before review.</CardDescription>
        </CardHeader>
        <CardContent>
          <MySubmissions />
        </CardContent>
      </Card>

      <Button variant="ghost" size="sm" asChild className="text-muted-foreground">
        <Link to="/templates">
          <ArrowLeft className="h-4 w-4" />
          Back to templates
        </Link>
      </Button>
    </div>
  );
}
