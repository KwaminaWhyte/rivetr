import { useState } from "react";
import { useParams } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  Folder,
  File,
  ChevronRight,
  Home,
  Loader2,
  Save,
  Trash2,
  RefreshCw,
  ArrowLeft,
} from "lucide-react";
import { api } from "@/lib/api";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
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
import { useBreadcrumb } from "@/lib/breadcrumb-context";
import type { FileEntry } from "@/lib/api/filesystem";

export function meta() {
  return [
    { title: "File Browser - Rivetr" },
    { name: "description", content: "Browse and edit files on remote server" },
  ];
}

function formatSize(bytes: number | null): string {
  if (bytes === null) return "-";
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)}MB`;
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return "-";
  try {
    return new Date(dateStr).toLocaleString();
  } catch {
    return dateStr;
  }
}

export default function ServerFilesPage() {
  const { id: serverId } = useParams<{ id: string }>();
  const queryClient = useQueryClient();

  useBreadcrumb([
    { label: "Servers", href: "/servers" },
    { label: "File Browser" },
  ]);

  const [currentPath, setCurrentPath] = useState("/");
  const [selectedFile, setSelectedFile] = useState<FileEntry | null>(null);
  const [fileContent, setFileContent] = useState<string>("");
  const [editedContent, setEditedContent] = useState<string>("");
  const [isEditing, setIsEditing] = useState(false);
  const [deletingPath, setDeletingPath] = useState<string | null>(null);

  // Build breadcrumb segments from path
  const pathSegments = currentPath
    .split("/")
    .filter(Boolean)
    .map((seg, i, arr) => ({
      label: seg,
      path: "/" + arr.slice(0, i + 1).join("/"),
    }));

  // Directory listing
  const {
    data: entries,
    isLoading,
    isError,
    error,
  } = useQuery({
    queryKey: ["server-files", serverId, currentPath],
    queryFn: () => api.browseFiles(serverId!, currentPath),
    enabled: !!serverId,
  });

  // File content query
  const { isLoading: loadingContent } = useQuery({
    queryKey: ["server-file-content", serverId, selectedFile?.path],
    queryFn: async () => {
      if (!selectedFile || !serverId) return null;
      const result = await api.readRemoteFile(serverId, selectedFile.path);
      setFileContent(result.content);
      setEditedContent(result.content);
      return result;
    },
    enabled: !!selectedFile && !!serverId && !selectedFile.is_dir,
  });

  // Write file mutation
  const writeMutation = useMutation({
    mutationFn: () =>
      api.writeRemoteFile(serverId!, selectedFile!.path, editedContent),
    onSuccess: () => {
      toast.success("File saved successfully");
      setFileContent(editedContent);
      setIsEditing(false);
      queryClient.invalidateQueries({
        queryKey: ["server-file-content", serverId, selectedFile?.path],
      });
    },
    onError: (err: Error) => {
      toast.error(err.message || "Failed to save file");
    },
  });

  // Delete mutation
  const deleteMutation = useMutation({
    mutationFn: (path: string) => api.deleteRemoteFile(serverId!, path),
    onSuccess: () => {
      toast.success("Deleted successfully");
      queryClient.invalidateQueries({
        queryKey: ["server-files", serverId, currentPath],
      });
      if (selectedFile?.path === deletingPath) {
        setSelectedFile(null);
        setFileContent("");
      }
      setDeletingPath(null);
    },
    onError: (err: Error) => {
      toast.error(err.message || "Failed to delete");
      setDeletingPath(null);
    },
  });

  function handleEntryClick(entry: FileEntry) {
    if (entry.is_dir) {
      setCurrentPath(entry.path);
      setSelectedFile(null);
      setFileContent("");
      setIsEditing(false);
    } else {
      setSelectedFile(entry);
      setIsEditing(false);
    }
  }

  function navigateUp() {
    const parts = currentPath.split("/").filter(Boolean);
    if (parts.length === 0) return;
    parts.pop();
    setCurrentPath(parts.length === 0 ? "/" : "/" + parts.join("/"));
    setSelectedFile(null);
    setFileContent("");
  }

  return (
    <div className="container py-6 space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">File Browser</h1>
        <Button
          variant="outline"
          size="sm"
          onClick={() =>
            queryClient.invalidateQueries({
              queryKey: ["server-files", serverId, currentPath],
            })
          }
        >
          <RefreshCw className="h-4 w-4 mr-1" />
          Refresh
        </Button>
      </div>

      {/* Breadcrumb */}
      <div className="flex items-center gap-1 text-sm text-muted-foreground">
        <button
          className="hover:text-foreground flex items-center gap-1"
          onClick={() => {
            setCurrentPath("/");
            setSelectedFile(null);
            setFileContent("");
          }}
        >
          <Home className="h-3 w-3" />
          /
        </button>
        {pathSegments.map((seg, i) => (
          <span key={seg.path} className="flex items-center gap-1">
            <ChevronRight className="h-3 w-3" />
            {i === pathSegments.length - 1 ? (
              <span className="text-foreground font-medium">{seg.label}</span>
            ) : (
              <button
                className="hover:text-foreground"
                onClick={() => {
                  setCurrentPath(seg.path);
                  setSelectedFile(null);
                }}
              >
                {seg.label}
              </button>
            )}
          </span>
        ))}
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Directory listing */}
        <Card>
          <CardHeader className="py-3 px-4">
            <div className="flex items-center justify-between">
              <CardTitle className="text-sm">
                {currentPath || "/"}
              </CardTitle>
              {currentPath !== "/" && (
                <Button size="sm" variant="ghost" onClick={navigateUp}>
                  <ArrowLeft className="h-4 w-4 mr-1" />
                  Up
                </Button>
              )}
            </div>
          </CardHeader>
          <CardContent className="p-0">
            {isLoading ? (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
              </div>
            ) : isError ? (
              <div className="px-4 py-6 text-center text-sm text-destructive">
                {(error as Error)?.message || "Failed to load directory"}
              </div>
            ) : !entries || entries.length === 0 ? (
              <div className="px-4 py-6 text-center text-sm text-muted-foreground">
                Empty directory
              </div>
            ) : (
              <div className="divide-y">
                {entries
                  .sort((a, b) => {
                    // Directories first
                    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
                    return a.name.localeCompare(b.name);
                  })
                  .map((entry) => (
                    <div
                      key={entry.path}
                      className={`flex items-center gap-3 px-4 py-2 cursor-pointer hover:bg-muted/50 transition-colors ${
                        selectedFile?.path === entry.path
                          ? "bg-muted"
                          : ""
                      }`}
                      onClick={() => handleEntryClick(entry)}
                    >
                      {entry.is_dir ? (
                        <Folder className="h-4 w-4 text-blue-500 shrink-0" />
                      ) : (
                        <File className="h-4 w-4 text-muted-foreground shrink-0" />
                      )}
                      <div className="flex-1 min-w-0">
                        <div className="text-sm font-medium truncate">
                          {entry.name}
                        </div>
                        <div className="text-xs text-muted-foreground flex gap-3">
                          {!entry.is_dir && (
                            <span>{formatSize(entry.size)}</span>
                          )}
                          <span>{formatDate(entry.modified)}</span>
                        </div>
                      </div>
                      <button
                        className="text-muted-foreground hover:text-destructive opacity-0 group-hover:opacity-100 shrink-0"
                        onClick={(e) => {
                          e.stopPropagation();
                          setDeletingPath(entry.path);
                        }}
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </button>
                    </div>
                  ))}
              </div>
            )}
          </CardContent>
        </Card>

        {/* File editor panel */}
        <Card>
          <CardHeader className="py-3 px-4">
            <div className="flex items-center justify-between">
              <CardTitle className="text-sm truncate">
                {selectedFile ? selectedFile.name : "No file selected"}
              </CardTitle>
              {selectedFile && !selectedFile.is_dir && (
                <div className="flex gap-2">
                  {isEditing ? (
                    <>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => {
                          setEditedContent(fileContent);
                          setIsEditing(false);
                        }}
                      >
                        Cancel
                      </Button>
                      <Button
                        size="sm"
                        disabled={writeMutation.isPending}
                        onClick={() => writeMutation.mutate()}
                      >
                        {writeMutation.isPending ? (
                          <Loader2 className="h-4 w-4 animate-spin" />
                        ) : (
                          <>
                            <Save className="h-4 w-4 mr-1" />
                            Save
                          </>
                        )}
                      </Button>
                    </>
                  ) : (
                    <>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => setIsEditing(true)}
                      >
                        Edit
                      </Button>
                      <Button
                        size="sm"
                        variant="ghost"
                        className="text-destructive hover:bg-destructive/10"
                        onClick={() => setDeletingPath(selectedFile.path)}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </>
                  )}
                </div>
              )}
            </div>
          </CardHeader>
          <CardContent className="p-0">
            {!selectedFile ? (
              <div className="flex items-center justify-center py-16 text-center px-4">
                <div>
                  <File className="h-10 w-10 text-muted-foreground mx-auto mb-3" />
                  <p className="text-sm text-muted-foreground">
                    Select a file to view or edit its contents
                  </p>
                </div>
              </div>
            ) : selectedFile.is_dir ? (
              <div className="flex items-center justify-center py-16 text-center px-4">
                <p className="text-sm text-muted-foreground">
                  Directory selected. Click a file to view its contents.
                </p>
              </div>
            ) : loadingContent ? (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
              </div>
            ) : (
              <Textarea
                value={isEditing ? editedContent : fileContent}
                onChange={(e) => setEditedContent(e.target.value)}
                readOnly={!isEditing}
                className="min-h-[400px] font-mono text-xs rounded-none border-0 border-t resize-none focus-visible:ring-0"
                spellCheck={false}
              />
            )}
          </CardContent>
        </Card>
      </div>

      {/* Delete confirmation */}
      <AlertDialog
        open={!!deletingPath}
        onOpenChange={(open) => {
          if (!open) setDeletingPath(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete?</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete{" "}
              <code className="text-sm font-mono bg-muted px-1 rounded">
                {deletingPath}
              </code>
              ? This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive hover:bg-destructive/90"
              onClick={() => {
                if (deletingPath) deleteMutation.mutate(deletingPath);
              }}
            >
              {deleteMutation.isPending ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                "Delete"
              )}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
