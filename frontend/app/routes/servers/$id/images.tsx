import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  Check,
  ChevronLeft,
  ChevronRight,
  Copy,
  Layers,
  Loader2,
  RefreshCw,
  Search,
  Trash2,
} from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { QueryError } from "@/components/query-error";
import { serverOpsApi } from "@/lib/api/server-ops";
import type { ServerImage } from "@/lib/api/server-ops";
import { ConfirmActionDialog } from "@/components/servers/confirm-action-dialog";
import { formatBytes, relativeTime } from "@/components/servers/server-ops-utils";
import { useServerContext } from "@/components/servers/server-context";

export function meta() {
  return [{ title: "Images - Rivetr" }];
}

function CopyId({ id, shortId }: { id: string; shortId: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      className="inline-flex items-center gap-1.5 font-mono text-xs text-muted-foreground hover:text-foreground"
      title="Copy full image id"
      onClick={async () => {
        try {
          await navigator.clipboard.writeText(id);
          setCopied(true);
          setTimeout(() => setCopied(false), 1500);
        } catch {
          toast.error("Could not copy to clipboard");
        }
      }}
    >
      {shortId}
      {copied ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
    </button>
  );
}

function Chip({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border bg-muted/40 px-3 py-1.5 text-sm">
      <span className="text-muted-foreground">{label}</span>{" "}
      <span className="font-semibold">{value}</span>
    </div>
  );
}

export default function ServerImagesPage() {
  const { serverId } = useServerContext();
  const queryClient = useQueryClient();
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(1);
  const [perPage, setPerPage] = useState(10);
  const [toDelete, setToDelete] = useState<ServerImage | null>(null);

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ["server-images", serverId],
    queryFn: () => serverOpsApi.listImages(serverId),
  });

  const deleteMutation = useMutation({
    mutationFn: (img: ServerImage) => serverOpsApi.deleteImage(serverId, img.id),
    onSuccess: () => {
      toast.success("Image removed");
      setToDelete(null);
      queryClient.invalidateQueries({ queryKey: ["server-images", serverId] });
    },
    onError: (e: Error) => toast.error(e.message || "Failed to remove image"),
  });

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    const images = data?.images ?? [];
    if (!q) return images;
    return images.filter(
      (i) =>
        i.repository.toLowerCase().includes(q) ||
        i.tag.toLowerCase().includes(q) ||
        i.id.includes(q),
    );
  }, [data, search]);

  const totalPages = Math.max(1, Math.ceil(filtered.length / perPage));
  const currentPage = Math.min(page, totalPages);
  const pageItems = filtered.slice((currentPage - 1) * perPage, currentPage * perPage);
  const start = filtered.length === 0 ? 0 : (currentPage - 1) * perPage + 1;
  const end = Math.min(currentPage * perPage, filtered.length);

  const reclaimPct =
    data && data.total_bytes > 0 ? Math.round((data.reclaimable_bytes / data.total_bytes) * 100) : 0;

  return (
    <Card>
      <CardHeader className="space-y-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <CardTitle className="flex items-center gap-2">
            <Layers className="h-5 w-5" />
            Images
          </CardTitle>
          <div className="flex flex-wrap items-center gap-2">
            {data && (
              <>
                <Chip label="Total" value={formatBytes(data.total_bytes)} />
                <Chip
                  label="Reclaimable"
                  value={`${formatBytes(data.reclaimable_bytes)} (${reclaimPct}%)`}
                />
              </>
            )}
            <Button variant="outline" size="sm" onClick={() => refetch()} disabled={isFetching}>
              <RefreshCw className={`h-3.5 w-3.5 ${isFetching ? "animate-spin" : ""}`} />
              Refresh
            </Button>
          </div>
        </div>
        <div className="relative max-w-sm">
          <Search className="absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            className="pl-8"
            placeholder="Search by repository, tag or ID"
            value={search}
            onChange={(e) => {
              setSearch(e.target.value);
              setPage(1);
            }}
          />
        </div>
      </CardHeader>
      <CardContent>
        {error ? (
          <QueryError what="images" error={error} onRetry={() => refetch()} />
        ) : isLoading ? (
          <div className="flex items-center justify-center py-12">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          </div>
        ) : filtered.length === 0 ? (
          <div className="py-12 text-center">
            <Layers className="mx-auto h-10 w-10 text-muted-foreground/50" />
            <p className="mt-2 font-medium">{search ? "No matching images" : "No images"}</p>
            <p className="text-sm text-muted-foreground">
              {search ? "Try a different search." : "This server has no container images yet."}
            </p>
          </div>
        ) : (
          <>
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Repository</TableHead>
                    <TableHead>Tag</TableHead>
                    <TableHead>Image ID</TableHead>
                    <TableHead>Created</TableHead>
                    <TableHead className="text-right">Size</TableHead>
                    <TableHead>Used by</TableHead>
                    <TableHead className="w-12 text-right">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {pageItems.map((img) => {
                    const unused = img.used_by.length === 0;
                    return (
                      <TableRow key={`${img.id}-${img.repository}-${img.tag}`}>
                        <TableCell className="max-w-[260px] truncate font-medium" title={img.repository}>
                          {img.repository}
                        </TableCell>
                        <TableCell className="max-w-[160px] truncate font-mono text-xs" title={img.tag}>
                          {img.tag}
                        </TableCell>
                        <TableCell>
                          <CopyId id={img.id} shortId={img.short_id} />
                        </TableCell>
                        <TableCell
                          className="whitespace-nowrap text-sm text-muted-foreground"
                          title={img.created_at ?? undefined}
                        >
                          {img.created_at ? relativeTime(img.created_at) : "-"}
                        </TableCell>
                        <TableCell className="whitespace-nowrap text-right text-sm">
                          {formatBytes(img.size_bytes)}
                        </TableCell>
                        <TableCell>
                          <div className="flex flex-wrap items-center gap-1">
                            {unused ? (
                              <Badge variant="secondary">Unused</Badge>
                            ) : (
                              img.used_by.map((n) => (
                                <Badge key={n} variant="outline" className="font-mono">
                                  {n}
                                </Badge>
                              ))
                            )}
                            {img.retained && (
                              <Badge variant="outline" title="Kept so deployments can roll back">
                                Rollback
                              </Badge>
                            )}
                          </div>
                        </TableCell>
                        <TableCell className="text-right">
                          {unused && (
                            <Button
                              variant="ghost"
                              size="sm"
                              className="text-destructive hover:text-destructive"
                              aria-label={`Delete ${img.repository}:${img.tag}`}
                              onClick={() => setToDelete(img)}
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          )}
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </div>
            <div className="flex flex-wrap items-center justify-between gap-3 border-t pt-4 text-sm">
              <span className="text-muted-foreground">
                {start}-{end} of {filtered.length}
              </span>
              <div className="flex items-center gap-2">
                <Select
                  value={String(perPage)}
                  onValueChange={(v) => {
                    setPerPage(Number(v));
                    setPage(1);
                  }}
                >
                  <SelectTrigger className="h-8 w-[110px]">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {[10, 25, 50, 100].map((n) => (
                      <SelectItem key={n} value={String(n)}>
                        {n} / page
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Button
                  variant="outline"
                  size="sm"
                  aria-label="Previous page"
                  disabled={currentPage <= 1}
                  onClick={() => setPage(currentPage - 1)}
                >
                  <ChevronLeft className="h-4 w-4" />
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  aria-label="Next page"
                  disabled={currentPage >= totalPages}
                  onClick={() => setPage(currentPage + 1)}
                >
                  <ChevronRight className="h-4 w-4" />
                </Button>
              </div>
            </div>
          </>
        )}
      </CardContent>

      <ConfirmActionDialog
        open={!!toDelete}
        onOpenChange={(o) => !o && setToDelete(null)}
        title="Delete image?"
        description={
          toDelete && (
            <>
              <p>
                <span className="font-mono">
                  {toDelete.repository}:{toDelete.tag}
                </span>{" "}
                ({formatBytes(toDelete.size_bytes)}) will be removed from this server, with every
                tag that points at it.
              </p>
              {toDelete.retained && (
                <p className="text-amber-600 dark:text-amber-400">
                  Rivetr keeps this image for rollbacks. Without it, rolling back to that deployment
                  needs a rebuild.
                </p>
              )}
            </>
          )
        }
        confirmLabel="Delete image"
        pending={deleteMutation.isPending}
        onConfirm={() => toDelete && deleteMutation.mutate(toDelete)}
      />
    </Card>
  );
}
