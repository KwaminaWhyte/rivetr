import { useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  ChevronRight,
  Download,
  File as FileIcon,
  FilePlus,
  FlaskConical,
  Folder,
  FolderPlus,
  HardDrive,
  Link2,
  Loader2,
  MoreHorizontal,
  Pencil,
  Plus,
  RefreshCw,
  Trash2,
  Upload,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Textarea } from "@/components/ui/textarea";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
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
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { appConfigApi, type FileTarget } from "@/lib/api";
import type { ContainerFileEntry } from "@/types/app-config";

/** Must match MAX_TRANSFER_BYTES in src/api/container_files.rs */
const MAX_UPLOAD_BYTES = 50 * 1024 * 1024;

function formatSize(bytes: number | null): string {
  if (bytes == null) return "";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = bytes / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v < 10 ? 1 : 0)} ${units[i]}`;
}

function formatTime(secs: number | null): string {
  if (secs == null) return "";
  return new Date(secs * 1000).toLocaleString();
}

function joinPath(dir: string, name: string): string {
  return dir === "/" ? `/${name}` : `${dir}/${name}`;
}

function parentOf(path: string): string {
  const idx = path.lastIndexOf("/");
  return idx <= 0 ? "/" : path.slice(0, idx);
}

function isValidName(name: string): boolean {
  const n = name.trim();
  return n !== "" && n !== "." && n !== ".." && !n.includes("/") && n.length <= 255;
}

interface ContainerFileBrowserProps {
  target: FileTarget;
  /** Shown when there is no container to browse (resource stopped, no container chosen). */
  unavailableMessage?: string | null;
  /** Rendered next to the path bar, e.g. a container picker for services. */
  toolbarExtra?: React.ReactNode;
  initialPath?: string;
}

/**
 * Browse and edit files inside a running container. Beta: listings come from `ls`
 * in the container, and transfers are capped at 50 MB.
 */
export function ContainerFileBrowser({
  target,
  unavailableMessage,
  toolbarExtra,
  initialPath = "/",
}: ContainerFileBrowserProps) {
  const queryClient = useQueryClient();
  const [path, setPath] = useState(initialPath);
  const [pathInput, setPathInput] = useState(initialPath);
  const [editing, setEditing] = useState<{ path: string; content: string; original: string } | null>(null);
  const [binaryFile, setBinaryFile] = useState<ContainerFileEntry | null>(null);
  const [creating, setCreating] = useState<"file" | "folder" | null>(null);
  const [newName, setNewName] = useState("");
  const [toDelete, setToDelete] = useState<ContainerFileEntry | null>(null);
  const uploadRef = useRef<HTMLInputElement>(null);

  const enabled = !unavailableMessage;
  const queryKey = ["container-files", target.kind, target.id, target.container ?? "", path];

  const listing = useQuery({
    queryKey,
    queryFn: () => appConfigApi.listFiles(target, path),
    enabled,
    retry: false,
  });

  const refresh = () => queryClient.invalidateQueries({ queryKey });

  const navigate = (next: string) => {
    setPath(next);
    setPathInput(next);
  };

  const openFile = useMutation({
    mutationFn: (entry: ContainerFileEntry) => appConfigApi.readFile(target, entry.path),
    onSuccess: (res, entry) => {
      if (res.binary) {
        setBinaryFile(entry);
      } else {
        setEditing({ path: res.path, content: res.content, original: res.content });
      }
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const saveFile = useMutation({
    mutationFn: (file: { path: string; content: string }) =>
      appConfigApi.writeFile(target, file.path, file.content),
    onSuccess: () => {
      toast.success("File saved");
      setEditing(null);
      refresh();
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const create = useMutation({
    mutationFn: async ({ kind, name }: { kind: "file" | "folder"; name: string }) => {
      const full = joinPath(path, name.trim());
      if (kind === "folder") {
        await appConfigApi.createFolder(target, full);
      } else {
        await appConfigApi.writeFile(target, full, "");
      }
      return { kind, full };
    },
    onSuccess: ({ kind, full }) => {
      toast.success(kind === "folder" ? "Folder created" : "File created");
      setCreating(null);
      setNewName("");
      refresh();
      if (kind === "file") setEditing({ path: full, content: "", original: "" });
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const remove = useMutation({
    mutationFn: (entry: ContainerFileEntry) => appConfigApi.deletePath(target, entry.path),
    onSuccess: () => {
      toast.success("Deleted");
      setToDelete(null);
      refresh();
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const upload = useMutation({
    mutationFn: (file: File) => appConfigApi.uploadFile(target, path, file),
    onSuccess: (_, file) => {
      toast.success(`Uploaded ${file.name}`);
      refresh();
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const download = useMutation({
    mutationFn: (entry: ContainerFileEntry) =>
      appConfigApi.downloadFile(target, entry.path, entry.name),
    onError: (e: Error) => toast.error(e.message),
  });

  const handleUploadPick = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (!file) return;
    if (file.size > MAX_UPLOAD_BYTES) {
      toast.error("Files larger than 50 MB cannot be uploaded");
      return;
    }
    upload.mutate(file);
  };

  const segments = path.split("/").filter(Boolean);
  const entries = listing.data?.entries ?? [];

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <HardDrive className="h-5 w-5" />
          Files
          <Badge variant="outline" className="gap-1 text-xs font-normal">
            <FlaskConical className="h-3 w-3" />
            Beta
          </Badge>
        </CardTitle>
        <CardDescription>
          Browse and edit files inside the running container. Changes are lost when the container
          is recreated unless the path is on a volume. Uploads and downloads are limited to 50 MB.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex flex-col gap-2 lg:flex-row lg:items-center">
          {toolbarExtra}
          <form
            className="flex min-w-0 flex-1 items-center gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              const next = pathInput.trim() || "/";
              navigate(next.startsWith("/") ? next : `/${next}`);
            }}
          >
            <Input
              aria-label="Path"
              value={pathInput}
              onChange={(e) => setPathInput(e.target.value)}
              className="font-mono text-sm"
              disabled={!enabled}
            />
          </form>
          <div className="flex items-center gap-2">
            <input ref={uploadRef} type="file" className="hidden" onChange={handleUploadPick} />
            <Button
              variant="outline"
              size="sm"
              className="gap-1"
              onClick={() => uploadRef.current?.click()}
              disabled={!enabled || upload.isPending}
            >
              {upload.isPending ? <Loader2 className="h-4 w-4 animate-spin" /> : <Upload className="h-4 w-4" />}
              Upload
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="outline" size="sm" className="gap-1" disabled={!enabled}>
                  <Plus className="h-4 w-4" />
                  New
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={() => setCreating("file")}>
                  <FilePlus className="mr-2 h-4 w-4" />
                  File
                </DropdownMenuItem>
                <DropdownMenuItem onClick={() => setCreating("folder")}>
                  <FolderPlus className="mr-2 h-4 w-4" />
                  Folder
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
            <Button
              variant="outline"
              size="sm"
              aria-label="Refresh"
              onClick={refresh}
              disabled={!enabled || listing.isFetching}
            >
              <RefreshCw className={`h-4 w-4 ${listing.isFetching ? "animate-spin" : ""}`} />
            </Button>
          </div>
        </div>

        <nav aria-label="Current folder" className="flex flex-wrap items-center gap-1 text-sm">
          <button
            type="button"
            className="rounded px-1 font-mono text-muted-foreground hover:bg-muted hover:text-foreground"
            onClick={() => navigate("/")}
          >
            /
          </button>
          {segments.map((seg, i) => {
            const to = "/" + segments.slice(0, i + 1).join("/");
            return (
              <span key={to} className="flex items-center gap-1">
                {i > 0 && <ChevronRight className="h-3 w-3 text-muted-foreground" />}
                <button
                  type="button"
                  className="rounded px-1 font-mono hover:bg-muted"
                  onClick={() => navigate(to)}
                >
                  {seg}
                </button>
              </span>
            );
          })}
        </nav>

        {unavailableMessage ? (
          <div className="rounded-lg border border-dashed p-8 text-center text-sm text-muted-foreground">
            {unavailableMessage}
          </div>
        ) : listing.isLoading ? (
          <div className="space-y-2">
            {Array.from({ length: 6 }).map((_, i) => (
              <Skeleton key={i} className="h-8 w-full" />
            ))}
          </div>
        ) : listing.isError ? (
          <div className="space-y-3 rounded-lg border border-dashed p-8 text-center text-sm">
            <p className="text-destructive">{(listing.error as Error).message}</p>
            {path !== "/" && (
              <Button variant="outline" size="sm" onClick={() => navigate(parentOf(path))}>
                Go up
              </Button>
            )}
          </div>
        ) : (
          <div className="overflow-x-auto rounded-md border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead className="hidden md:table-cell">Perms</TableHead>
                  <TableHead className="hidden md:table-cell">Owner</TableHead>
                  <TableHead className="hidden md:table-cell">Group</TableHead>
                  <TableHead className="text-right">Size</TableHead>
                  <TableHead className="hidden lg:table-cell">Modified</TableHead>
                  <TableHead className="w-12">
                    <span className="sr-only">Actions</span>
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {path !== "/" && (
                  <TableRow className="cursor-pointer" onClick={() => navigate(parentOf(path))}>
                    <TableCell colSpan={7} className="font-mono text-sm text-muted-foreground">
                      ..
                    </TableCell>
                  </TableRow>
                )}
                {entries.length === 0 && (
                  <TableRow>
                    <TableCell colSpan={7} className="py-8 text-center text-sm text-muted-foreground">
                      This folder is empty.
                    </TableCell>
                  </TableRow>
                )}
                {entries.map((entry) => {
                  const openable = entry.kind === "file" || entry.kind === "symlink";
                  return (
                    <TableRow key={entry.path}>
                      <TableCell>
                        <button
                          type="button"
                          className="flex items-center gap-2 text-left font-mono text-sm hover:underline disabled:no-underline"
                          onClick={() =>
                            entry.is_dir ? navigate(entry.path) : openable && openFile.mutate(entry)
                          }
                          disabled={!entry.is_dir && !openable}
                        >
                          {entry.is_dir ? (
                            <Folder className="h-4 w-4 shrink-0 text-blue-500" />
                          ) : entry.kind === "symlink" ? (
                            <Link2 className="h-4 w-4 shrink-0 text-muted-foreground" />
                          ) : (
                            <FileIcon className="h-4 w-4 shrink-0 text-muted-foreground" />
                          )}
                          <span className="break-all">{entry.name}</span>
                          {entry.link_target && (
                            <span className="text-xs text-muted-foreground">to {entry.link_target}</span>
                          )}
                        </button>
                      </TableCell>
                      <TableCell className="hidden font-mono text-xs md:table-cell">{entry.permissions}</TableCell>
                      <TableCell className="hidden text-sm md:table-cell">{entry.owner}</TableCell>
                      <TableCell className="hidden text-sm md:table-cell">{entry.group}</TableCell>
                      <TableCell className="text-right text-sm tabular-nums">
                        {entry.is_dir ? "" : formatSize(entry.size)}
                      </TableCell>
                      <TableCell className="hidden text-sm text-muted-foreground lg:table-cell">
                        {formatTime(entry.modified)}
                      </TableCell>
                      <TableCell>
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button variant="ghost" size="sm" className="h-7 w-7 p-0" aria-label={`Actions for ${entry.name}`}>
                              <MoreHorizontal className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            {entry.is_dir ? (
                              <DropdownMenuItem onClick={() => navigate(entry.path)}>
                                <Folder className="mr-2 h-4 w-4" />
                                Open
                              </DropdownMenuItem>
                            ) : (
                              <>
                                <DropdownMenuItem disabled={!openable} onClick={() => openFile.mutate(entry)}>
                                  <Pencil className="mr-2 h-4 w-4" />
                                  Open
                                </DropdownMenuItem>
                                <DropdownMenuItem disabled={!openable} onClick={() => download.mutate(entry)}>
                                  <Download className="mr-2 h-4 w-4" />
                                  Download
                                </DropdownMenuItem>
                              </>
                            )}
                            <DropdownMenuSeparator />
                            <DropdownMenuItem
                              className="text-destructive focus:text-destructive"
                              onClick={() => setToDelete(entry)}
                            >
                              <Trash2 className="mr-2 h-4 w-4" />
                              Delete
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          </div>
        )}
        {openFile.isPending && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            Opening file...
          </p>
        )}
      </CardContent>

      {/* Editor */}
      <Dialog open={editing !== null} onOpenChange={(open) => !open && setEditing(null)}>
        <DialogContent className="sm:max-w-4xl">
          <DialogHeader>
            <DialogTitle className="break-all font-mono text-base">{editing?.path}</DialogTitle>
            <DialogDescription>Files up to 1 MB can be edited here.</DialogDescription>
          </DialogHeader>
          <Textarea
            aria-label="File content"
            value={editing?.content ?? ""}
            onChange={(e) => editing && setEditing({ ...editing, content: e.target.value })}
            className="min-h-[50vh] font-mono text-sm"
            spellCheck={false}
          />
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditing(null)}>
              Close
            </Button>
            <Button
              onClick={() => editing && saveFile.mutate({ path: editing.path, content: editing.content })}
              disabled={!editing || editing.content === editing.original || saveFile.isPending}
            >
              {saveFile.isPending ? "Saving..." : "Save"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Binary file notice */}
      <Dialog open={binaryFile !== null} onOpenChange={(open) => !open && setBinaryFile(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle className="break-all font-mono text-base">{binaryFile?.name}</DialogTitle>
            <DialogDescription>This is a binary file and cannot be shown as text.</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setBinaryFile(null)}>
              Close
            </Button>
            <Button
              onClick={() => {
                if (binaryFile) download.mutate(binaryFile);
                setBinaryFile(null);
              }}
            >
              <Download className="mr-2 h-4 w-4" />
              Download
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* New file / folder */}
      <Dialog
        open={creating !== null}
        onOpenChange={(open) => {
          if (!open) {
            setCreating(null);
            setNewName("");
          }
        }}
      >
        <DialogContent>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              if (creating && isValidName(newName)) create.mutate({ kind: creating, name: newName });
            }}
            className="space-y-4"
          >
            <DialogHeader>
              <DialogTitle>{creating === "folder" ? "New folder" : "New file"}</DialogTitle>
              <DialogDescription className="break-all">
                Created in <span className="font-mono">{path}</span>
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-2">
              <Label htmlFor="new-entry-name">Name</Label>
              <Input
                id="new-entry-name"
                autoFocus
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                placeholder={creating === "folder" ? "config" : "notes.txt"}
              />
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setCreating(null)}>
                Cancel
              </Button>
              <Button type="submit" disabled={!isValidName(newName) || create.isPending}>
                {create.isPending ? "Creating..." : "Create"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Delete confirm */}
      <AlertDialog open={toDelete !== null} onOpenChange={(open) => !open && setToDelete(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete {toDelete?.is_dir ? "folder" : "file"}?</AlertDialogTitle>
            <AlertDialogDescription className="break-all">
              <span className="font-mono">{toDelete?.path}</span>
              {toDelete?.is_dir ? " and everything in it" : ""} will be removed from the
              container. This cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={(e) => {
                e.preventDefault();
                if (toDelete) remove.mutate(toDelete);
              }}
              disabled={remove.isPending}
              className="bg-destructive text-white hover:bg-destructive/90"
            >
              {remove.isPending ? "Deleting..." : "Delete"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </Card>
  );
}
