import { useEffect, useState } from "react";
import { Link } from "react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Box, Database, Layers, MoreVertical, Pencil, Plus, Rocket, Tags, Trash2 } from "lucide-react";
import {
  DataTable,
  EmptyState,
  PageHeader,
  ResourceStatus,
  SectionCard,
  type DataColumn,
} from "@/components/page";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { QueryError } from "@/components/query-error";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { ColorSwatches, TAG_COLORS, TagChip } from "@/components/tags/tag-chip";
import { useTags } from "@/components/tags/use-tags";
import { useBreadcrumb } from "@/lib/breadcrumb-context";
import { useTeamContext } from "@/lib/team-context";
import {
  tagsApi,
  type TagDeployResult,
  type TaggableKind,
  type TagWithCount,
} from "@/lib/api/tags";

export function meta() {
  return [
    { title: "Tags - Rivetr" },
    { name: "description", content: "Group apps, databases and services with tags" },
  ];
}

const KIND_ICON: Record<TaggableKind, typeof Box> = {
  app: Box,
  database: Database,
  service: Layers,
};
const KIND_PATH: Record<TaggableKind, string> = {
  app: "apps",
  database: "databases",
  service: "services",
};

const DEPLOY_LABEL: Record<TagDeployResult["status"], string> = {
  queued: "Queued",
  pending_approval: "Waiting for approval",
  scheduled: "Scheduled",
  skipped: "Skipped",
  failed: "Failed",
};

function TagFormDialog({
  open,
  onOpenChange,
  initial,
  title,
  submitLabel,
  pending,
  onSubmit,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  initial: { name: string; color: string };
  title: string;
  submitLabel: string;
  pending: boolean;
  onSubmit: (v: { name: string; color: string }) => void;
}) {
  const [name, setName] = useState(initial.name);
  const [color, setColor] = useState(initial.color);
  useEffect(() => {
    if (open) {
      setName(initial.name);
      setColor(initial.color);
    }
  }, [open, initial.name, initial.color]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            onSubmit({ name: name.trim(), color });
          }}
        >
          <DialogHeader>
            <DialogTitle>{title}</DialogTitle>
            <DialogDescription>
              Letters, digits, spaces and - _ . : / are allowed, up to 40 characters.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="tag-name">Name</Label>
              <Input
                id="tag-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                maxLength={40}
                placeholder="production"
                autoFocus
                required
              />
            </div>
            <div className="space-y-2">
              <Label>Color</Label>
              <ColorSwatches value={color} onChange={setColor} />
            </div>
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              Preview <TagChip name={name.trim() || "tag"} color={color} />
            </div>
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button type="submit" disabled={pending || !name.trim()}>
              {submitLabel}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function TagResourcesSheet({ tag, onClose }: { tag: TagWithCount | null; onClose: () => void }) {
  const { data = [], isLoading } = useQuery({
    queryKey: ["tag-resources", tag?.id],
    queryFn: () => tagsApi.resources(tag!.id),
    enabled: !!tag,
  });
  return (
    <Sheet open={!!tag} onOpenChange={(o) => !o && onClose()}>
      <SheetContent side="right" className="w-full gap-0 sm:max-w-md">
        {tag && (
          <>
            <SheetHeader className="border-b p-5">
              <SheetTitle className="flex items-center gap-2">
                <TagChip name={tag.name} color={tag.color} />
              </SheetTitle>
              <SheetDescription>Resources carrying this tag</SheetDescription>
            </SheetHeader>
            <div className="flex-1 overflow-y-auto p-3">
              {isLoading ? (
                <p className="p-2 text-sm text-muted-foreground">Loading...</p>
              ) : data.length === 0 ? (
                <EmptyState
                  compact
                  icon={<Tags />}
                  title="Nothing tagged yet"
                  description="Add this tag from the Tags card on an app, database or service."
                />
              ) : (
                <ul className="divide-y">
                  {data.map((r) => {
                    const Icon = KIND_ICON[r.resource_type];
                    return (
                      <li key={`${r.resource_type}:${r.id}`}>
                        <Link
                          to={`/${KIND_PATH[r.resource_type]}/${r.id}`}
                          className="flex items-center gap-3 rounded-md px-2 py-2.5 hover:bg-muted/60"
                        >
                          <Icon className="size-4 shrink-0 text-muted-foreground" />
                          <span className="min-w-0 flex-1 truncate text-sm font-medium">{r.name}</span>
                          {r.status && <ResourceStatus status={r.status} size="sm" />}
                        </Link>
                      </li>
                    );
                  })}
                </ul>
              )}
            </div>
          </>
        )}
      </SheetContent>
    </Sheet>
  );
}

export default function TagsPage() {
  const { setItems } = useBreadcrumb();
  const { currentTeamId } = useTeamContext();
  const queryClient = useQueryClient();
  const { data: tags = [], isLoading, error, refetch } = useTags(currentTeamId);

  const [creating, setCreating] = useState(false);
  const [editing, setEditing] = useState<TagWithCount | null>(null);
  const [deleting, setDeleting] = useState<TagWithCount | null>(null);
  const [deploying, setDeploying] = useState<TagWithCount | null>(null);
  const [viewing, setViewing] = useState<TagWithCount | null>(null);
  const [deployResults, setDeployResults] = useState<TagDeployResult[] | null>(null);

  useEffect(() => {
    setItems([{ label: "Tags" }]);
  }, [setItems]);

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ["tags"] });
    queryClient.invalidateQueries({ queryKey: ["tag-assignments"] });
  };

  const create = useMutation({
    mutationFn: (v: { name: string; color: string }) =>
      tagsApi.create({ ...v, team_id: currentTeamId }),
    onSuccess: () => {
      toast.success("Tag created");
      setCreating(false);
      invalidate();
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const update = useMutation({
    mutationFn: (v: { id: string; name: string; color: string }) =>
      tagsApi.update(v.id, { name: v.name, color: v.color }),
    onSuccess: () => {
      toast.success("Tag updated");
      setEditing(null);
      invalidate();
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const remove = useMutation({
    mutationFn: (id: string) => tagsApi.remove(id),
    onSuccess: () => {
      toast.success("Tag deleted");
      setDeleting(null);
      invalidate();
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const deploy = useMutation({
    mutationFn: (id: string) => tagsApi.deploy(id),
    onSuccess: (res) => {
      setDeploying(null);
      setDeployResults(res.results);
      const queued = res.results.filter((r) => r.status === "queued").length;
      toast.success(`${queued} of ${res.results.length} resources queued`);
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const columns: DataColumn<TagWithCount>[] = [
    {
      id: "name",
      header: "Tag",
      cell: (t) => <TagChip name={t.name} color={t.color} />,
    },
    {
      id: "scope",
      header: "Scope",
      showFrom: "sm",
      cell: (t) => (
        <span className="text-muted-foreground">{t.team_id ? "Team" : "Global"}</span>
      ),
    },
    {
      id: "count",
      header: "Resources",
      cell: (t) => <span className="tabular-nums">{t.resource_count}</span>,
    },
    {
      id: "actions",
      header: <span className="sr-only">Actions</span>,
      className: "w-12 text-right",
      cell: (t) => (
        <div onClick={(e) => e.stopPropagation()}>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon-sm" aria-label={`Actions for ${t.name}`}>
                <MoreVertical />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={() => setDeploying(t)} disabled={t.resource_count === 0}>
                <Rocket className="mr-2 size-4" />
                Deploy all with tag
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => setEditing(t)}>
                <Pencil className="mr-2 size-4" />
                Rename or recolor
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                className="text-destructive focus:text-destructive"
                onClick={() => setDeleting(t)}
              >
                <Trash2 className="mr-2 size-4" />
                Delete
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      ),
    },
  ];

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<Tags />}
        title="Tags"
        description="Label apps, databases and services, filter lists and the canvas by tag, and deploy everything with a tag at once."
        actions={
          <Button onClick={() => setCreating(true)}>
            <Plus className="size-4" />
            New tag
          </Button>
        }
      />

      {error && <QueryError error={error} onRetry={() => refetch()} />}

      <DataTable
        columns={columns}
        rows={tags}
        getRowKey={(t) => t.id}
        loading={isLoading}
        onRowClick={(t) => setViewing(t)}
        empty={
          <EmptyState
            icon={<Tags />}
            title="No tags yet"
            description="Create a tag, then add it from the Tags card on any app, database or service."
            action={
              <Button onClick={() => setCreating(true)}>
                <Plus className="size-4" />
                New tag
              </Button>
            }
          />
        }
      />

      {deployResults && (
        <SectionCard
          title="Last deploy by tag"
          actions={
            <Button variant="ghost" size="sm" onClick={() => setDeployResults(null)}>
              Dismiss
            </Button>
          }
          flush
        >
          <ul className="divide-y text-sm">
            {deployResults.map((r) => (
              <li key={`${r.resource_type}:${r.id}`} className="flex flex-wrap items-center gap-2 px-4 py-2.5">
                <Link to={`/${KIND_PATH[r.resource_type]}/${r.id}`} className="font-medium hover:underline">
                  {r.name}
                </Link>
                <span className="text-muted-foreground">{DEPLOY_LABEL[r.status]}</span>
                {r.message && <span className="text-xs text-muted-foreground">({r.message})</span>}
              </li>
            ))}
          </ul>
        </SectionCard>
      )}

      <TagFormDialog
        open={creating}
        onOpenChange={setCreating}
        initial={{ name: "", color: TAG_COLORS[tags.length % TAG_COLORS.length] }}
        title="New tag"
        submitLabel="Create tag"
        pending={create.isPending}
        onSubmit={(v) => create.mutate(v)}
      />
      <TagFormDialog
        open={!!editing}
        onOpenChange={(o) => !o && setEditing(null)}
        initial={{ name: editing?.name ?? "", color: editing?.color ?? TAG_COLORS[0] }}
        title="Edit tag"
        submitLabel="Save"
        pending={update.isPending}
        onSubmit={(v) => editing && update.mutate({ id: editing.id, ...v })}
      />
      <ConfirmDialog
        open={!!deleting}
        onOpenChange={(o) => !o && setDeleting(null)}
        title={`Delete tag "${deleting?.name ?? ""}"?`}
        description="The tag is removed from every resource. The resources themselves are not changed."
        resourceName={deleting?.name}
        isPending={remove.isPending}
        onConfirm={() => {
          if (deleting) remove.mutate(deleting.id);
        }}
      />
      <ConfirmDialog
        open={!!deploying}
        onOpenChange={(o) => !o && setDeploying(null)}
        title={`Deploy everything tagged "${deploying?.name ?? ""}"?`}
        description="Every app and service with this tag is deployed. Team roles, approvals and freeze windows apply to each one. Databases are skipped."
        confirmLabel="Deploy"
        destructive={false}
        mode="simple"
        isPending={deploy.isPending}
        onConfirm={() => {
          if (deploying) deploy.mutate(deploying.id);
        }}
      />
      <TagResourcesSheet tag={viewing} onClose={() => setViewing(null)} />
    </div>
  );
}
