import { useMemo, useState } from "react";
import { Link } from "react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Check, Plus, Tags } from "lucide-react";
import { SectionCard } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { tagsApi, type Tag, type TaggableKind } from "@/lib/api/tags";
import { cn } from "@/lib/utils";
import { TAG_COLORS, TagChip } from "./tag-chip";
import { useTags } from "./use-tags";

/**
 * "Tags" card for an app, database or service: shows the resource's tags as
 * chips and lets the user attach, detach or create tags. Team tags only fit
 * resources of the same team; global tags fit everything.
 */
export function ResourceTagsCard({
  kind,
  resourceId,
  teamId,
}: {
  kind: TaggableKind;
  resourceId: string;
  teamId: string | null | undefined;
}) {
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [newName, setNewName] = useState("");

  const tagsQuery = useTags(teamId ?? null);
  const allTags = tagsQuery.data ?? [];
  // The API already returns the tags this resource may use (its team's plus the
  // global ones) and drops the rest. Filtering again here hid every tag from a
  // resource whose team_id is null, which is most pre-teams resources.
  const usable = allTags;

  const assignmentsKey = ["tag-assignments", "resource", kind, resourceId];
  const { data: assignments = [], isLoading } = useQuery({
    queryKey: assignmentsKey,
    queryFn: () => tagsApi.assignments({ resourceType: kind, resourceId }),
  });
  const attachedIds = useMemo(() => new Set(assignments.map((a) => a.tag_id)), [assignments]);
  const attached = usable.filter((t) => attachedIds.has(t.id));

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: assignmentsKey });
    queryClient.invalidateQueries({ queryKey: ["tag-assignments"] });
    queryClient.invalidateQueries({ queryKey: ["tags"] });
  };

  const setTags = useMutation({
    mutationFn: (ids: string[]) => tagsApi.setForResource(kind, resourceId, ids),
    onSuccess: invalidate,
    onError: (e: Error) => toast.error(e.message),
  });

  const createTag = useMutation({
    mutationFn: async (name: string) => {
      const color = TAG_COLORS[allTags.length % TAG_COLORS.length];
      const tag: Tag = await tagsApi.create({ name, color, team_id: teamId ?? null });
      return tagsApi.setForResource(kind, resourceId, [...attachedIds, tag.id]);
    },
    onSuccess: () => {
      setNewName("");
      invalidate();
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const toggle = (id: string) => {
    const next = new Set(attachedIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setTags.mutate([...next]);
  };

  const trimmed = newName.trim();
  const exists = usable.some((t) => t.name.toLowerCase() === trimmed.toLowerCase());

  return (
    <SectionCard
      id="tags"
      title="Tags"
      description={
        <>
          Group resources for filtering and bulk deploys. Manage all tags on the{" "}
          <Link to="/tags" className="underline underline-offset-2 hover:text-foreground">
            Tags page
          </Link>
          .
        </>
      }
    >
      <div className="flex flex-wrap items-center gap-2">
        {!isLoading && attached.length === 0 && (
          <span className="text-sm text-muted-foreground">No tags yet.</span>
        )}
        {attached.map((t) => (
          <TagChip key={t.id} name={t.name} color={t.color} onRemove={() => toggle(t.id)} />
        ))}
        <Popover open={open} onOpenChange={setOpen}>
          <PopoverTrigger asChild>
            <Button variant="outline" size="sm" disabled={setTags.isPending}>
              <Plus className="size-3.5" />
              {attached.length === 0 && usable.length > 0 ? "Select tag" : "Add tag"}
            </Button>
          </PopoverTrigger>
          <PopoverContent align="start" className="w-64 p-2">
            <div className="max-h-56 space-y-0.5 overflow-y-auto">
              {tagsQuery.isLoading && (
                <p className="px-2 py-1.5 text-xs text-muted-foreground">Loading tags…</p>
              )}
              {tagsQuery.isError && (
                <p className="px-2 py-1.5 text-xs text-destructive">
                  Could not load tags. Try again below.
                </p>
              )}
              {!tagsQuery.isLoading && !tagsQuery.isError && usable.length === 0 && (
                <p className="px-2 py-1.5 text-xs text-muted-foreground">
                  No tags yet. Create one below.
                </p>
              )}
              {usable.map((t) => {
                const on = attachedIds.has(t.id);
                return (
                  <button
                    key={t.id}
                    type="button"
                    onClick={() => toggle(t.id)}
                    className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm hover:bg-muted"
                    aria-pressed={on}
                  >
                    <span
                      aria-hidden="true"
                      className="size-2.5 shrink-0 rounded-full"
                      style={{ backgroundColor: t.color }}
                    />
                    <span className="min-w-0 flex-1 truncate">{t.name}</span>
                    <Check className={cn("size-4", on ? "opacity-100" : "opacity-0")} />
                  </button>
                );
              })}
            </div>
            <form
              className="mt-2 flex items-center gap-1.5 border-t pt-2"
              onSubmit={(e) => {
                e.preventDefault();
                if (trimmed && !exists) createTag.mutate(trimmed);
              }}
            >
              <Input
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                placeholder="New tag"
                aria-label="New tag name"
                maxLength={40}
                className="h-8"
              />
              <Button
                type="submit"
                size="sm"
                disabled={!trimmed || exists || createTag.isPending}
                aria-label="Create and add tag"
              >
                <Tags className="size-3.5" />
              </Button>
            </form>
          </PopoverContent>
        </Popover>
      </div>
    </SectionCard>
  );
}
