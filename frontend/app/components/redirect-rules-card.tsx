import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ArrowRight, Plus, Trash2, Pencil, GitCompare } from "lucide-react";
import api from "@/lib/api";
import type {
  AppRedirectRule,
  CreateRedirectRuleRequest,
  UpdateRedirectRuleRequest,
} from "@/types/api";

interface RedirectRulesCardProps {
  appId: string;
  token?: string;
}

const EMPTY_FORM: CreateRedirectRuleRequest = {
  source_pattern: "",
  destination: "",
  is_permanent: false,
  is_enabled: true,
  sort_order: 0,
};

export function RedirectRulesCard({ appId, token }: RedirectRulesCardProps) {
  const queryClient = useQueryClient();
  const [showDialog, setShowDialog] = useState(false);
  const [editingRule, setEditingRule] = useState<AppRedirectRule | null>(null);
  const [form, setForm] = useState<CreateRedirectRuleRequest>(EMPTY_FORM);

  // Fetch rules
  const { data: rules = [], isLoading } = useQuery({
    queryKey: ["redirect-rules", appId],
    queryFn: () => api.getRedirectRules(appId, token),
  });

  const invalidate = () =>
    queryClient.invalidateQueries({ queryKey: ["redirect-rules", appId] });

  // Create mutation
  const createMutation = useMutation({
    mutationFn: (data: CreateRedirectRuleRequest) =>
      api.createRedirectRule(appId, data, token),
    onSuccess: () => {
      toast.success("Redirect rule created");
      invalidate();
      closeDialog();
    },
    onError: (error: Error) =>
      toast.error(error.message || "Failed to create redirect rule"),
  });

  // Update mutation
  const updateMutation = useMutation({
    mutationFn: ({
      ruleId,
      data,
    }: {
      ruleId: string;
      data: UpdateRedirectRuleRequest;
    }) => api.updateRedirectRule(appId, ruleId, data, token),
    onSuccess: () => {
      toast.success("Redirect rule updated");
      invalidate();
      closeDialog();
    },
    onError: (error: Error) =>
      toast.error(error.message || "Failed to update redirect rule"),
  });

  // Delete mutation
  const deleteMutation = useMutation({
    mutationFn: (ruleId: string) => api.deleteRedirectRule(appId, ruleId, token),
    onSuccess: () => {
      toast.success("Redirect rule deleted");
      invalidate();
    },
    onError: (error: Error) =>
      toast.error(error.message || "Failed to delete redirect rule"),
  });

  // Toggle enabled mutation
  const toggleMutation = useMutation({
    mutationFn: ({ ruleId, enabled }: { ruleId: string; enabled: boolean }) =>
      api.updateRedirectRule(appId, ruleId, { is_enabled: enabled }, token),
    onSuccess: () => invalidate(),
    onError: (error: Error) =>
      toast.error(error.message || "Failed to toggle redirect rule"),
  });

  function openCreate() {
    setEditingRule(null);
    setForm(EMPTY_FORM);
    setShowDialog(true);
  }

  function openEdit(rule: AppRedirectRule) {
    setEditingRule(rule);
    setForm({
      source_pattern: rule.source_pattern,
      destination: rule.destination,
      is_permanent: rule.is_permanent !== 0,
      is_enabled: rule.is_enabled !== 0,
      sort_order: rule.sort_order,
    });
    setShowDialog(true);
  }

  function closeDialog() {
    setShowDialog(false);
    setEditingRule(null);
    setForm(EMPTY_FORM);
  }

  function handleSubmit() {
    if (!form.source_pattern.trim()) {
      toast.error("Source pattern is required");
      return;
    }
    if (!form.destination.trim()) {
      toast.error("Destination is required");
      return;
    }

    if (editingRule) {
      updateMutation.mutate({ ruleId: editingRule.id, data: form });
    } else {
      createMutation.mutate(form);
    }
  }

  const isSaving = createMutation.isPending || updateMutation.isPending;

  return (
    <>
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <GitCompare className="h-5 w-5" />
                URL Redirect Rules
              </CardTitle>
              <CardDescription className="mt-1">
                Regex-based HTTP redirects enforced at the proxy level — no
                application code changes required. Rules are evaluated in order
                before the request is forwarded.
              </CardDescription>
            </div>
            <Button size="sm" onClick={openCreate} className="shrink-0">
              <Plus className="h-4 w-4 mr-1" />
              Add Rule
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="animate-pulse h-20 bg-muted rounded" />
          ) : rules.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground text-sm">
              No redirect rules configured. Click{" "}
              <strong>Add Rule</strong> to get started.
            </div>
          ) : (
            <div className="space-y-3">
              {rules.map((rule) => (
                <div
                  key={rule.id}
                  className="flex items-center gap-3 p-3 rounded-lg border bg-card"
                >
                  {/* Enable toggle */}
                  <Switch
                    checked={rule.is_enabled !== 0}
                    onCheckedChange={(checked) =>
                      toggleMutation.mutate({ ruleId: rule.id, enabled: checked })
                    }
                    className="shrink-0"
                  />

                  {/* Pattern + destination */}
                  <div className="flex-1 min-w-0 flex items-center gap-2 flex-wrap">
                    <code className="text-xs px-1.5 py-0.5 bg-muted rounded font-mono truncate max-w-[200px]">
                      {rule.source_pattern}
                    </code>
                    <ArrowRight className="h-4 w-4 text-muted-foreground shrink-0" />
                    <code className="text-xs px-1.5 py-0.5 bg-muted rounded font-mono truncate max-w-[200px]">
                      {rule.destination}
                    </code>
                    <Badge
                      variant={rule.is_permanent !== 0 ? "default" : "secondary"}
                      className="text-xs shrink-0"
                    >
                      {rule.is_permanent !== 0 ? "301" : "302"}
                    </Badge>
                  </div>

                  {/* Actions */}
                  <div className="flex items-center gap-1 shrink-0">
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-8 w-8"
                      onClick={() => openEdit(rule)}
                    >
                      <Pencil className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-8 w-8 text-destructive hover:text-destructive"
                      onClick={() => deleteMutation.mutate(rule.id)}
                      disabled={deleteMutation.isPending}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Create / Edit Dialog */}
      <Dialog open={showDialog} onOpenChange={(open) => !open && closeDialog()}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>
              {editingRule ? "Edit Redirect Rule" : "Add Redirect Rule"}
            </DialogTitle>
            <DialogDescription>
              Use regex patterns for the source. Capture groups (e.g.{" "}
              <code className="text-xs bg-muted px-1 rounded">$1</code>) can be
              referenced in the destination.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-2">
            {/* Source Pattern */}
            <div className="space-y-2">
              <Label htmlFor="source-pattern">
                Source Pattern{" "}
                <span className="text-muted-foreground text-xs">(regex)</span>
              </Label>
              <Input
                id="source-pattern"
                value={form.source_pattern}
                onChange={(e) =>
                  setForm((f) => ({ ...f, source_pattern: e.target.value }))
                }
                placeholder="^/old-path(.*)"
                className="font-mono text-sm"
              />
              <p className="text-xs text-muted-foreground">
                Matched against the request path. Example:{" "}
                <code className="bg-muted px-1 rounded">^/blog/(.*)</code>
              </p>
            </div>

            {/* Destination */}
            <div className="space-y-2">
              <Label htmlFor="destination">Destination</Label>
              <Input
                id="destination"
                value={form.destination}
                onChange={(e) =>
                  setForm((f) => ({ ...f, destination: e.target.value }))
                }
                placeholder="/new-path$1"
                className="font-mono text-sm"
              />
              <p className="text-xs text-muted-foreground">
                Use{" "}
                <code className="bg-muted px-1 rounded">$1</code>,{" "}
                <code className="bg-muted px-1 rounded">$2</code> for capture
                groups from the source pattern.
              </p>
            </div>

            {/* Permanent toggle */}
            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label htmlFor="is-permanent">Permanent (301)</Label>
                <p className="text-xs text-muted-foreground">
                  Off = 302 temporary, On = 301 permanent
                </p>
              </div>
              <Switch
                id="is-permanent"
                checked={form.is_permanent ?? false}
                onCheckedChange={(checked) =>
                  setForm((f) => ({ ...f, is_permanent: checked }))
                }
              />
            </div>

            {/* Enabled toggle */}
            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label htmlFor="is-enabled">Enabled</Label>
                <p className="text-xs text-muted-foreground">
                  Disabled rules are stored but not applied
                </p>
              </div>
              <Switch
                id="is-enabled"
                checked={form.is_enabled ?? true}
                onCheckedChange={(checked) =>
                  setForm((f) => ({ ...f, is_enabled: checked }))
                }
              />
            </div>

            {/* Sort order */}
            <div className="space-y-2">
              <Label htmlFor="sort-order">
                Priority{" "}
                <span className="text-muted-foreground text-xs">
                  (lower = evaluated first)
                </span>
              </Label>
              <Input
                id="sort-order"
                type="number"
                value={form.sort_order ?? 0}
                onChange={(e) =>
                  setForm((f) => ({
                    ...f,
                    sort_order: parseInt(e.target.value, 10) || 0,
                  }))
                }
              />
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={closeDialog}>
              Cancel
            </Button>
            <Button onClick={handleSubmit} disabled={isSaving}>
              {isSaving
                ? "Saving..."
                : editingRule
                ? "Update Rule"
                : "Create Rule"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
