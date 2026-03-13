import { useRef, useState } from "react";
import { useOutletContext } from "react-router";
import { useMutation } from "@tanstack/react-query";
import type { ManagedDatabase } from "@/types/api";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { toast } from "sonner";
import { api } from "@/lib/api";
import { Upload, FileUp, Info, CheckCircle2 } from "lucide-react";

export function meta() {
  return [
    { title: "Import Database Dump - Rivetr" },
    { name: "description", content: "Import a SQL or archive dump into your database" },
  ];
}

interface OutletContext {
  database: ManagedDatabase;
}

// Supported formats per database type
const SUPPORTED_FORMATS: Record<string, string[]> = {
  postgres: [".sql (plain SQL via psql)", ".dump / .custom (pg_restore custom format)"],
  mysql: [".sql (mysqldump plain SQL)"],
  mariadb: [".sql (mariadb-dump plain SQL)"],
  mongodb: [".archive.gz (mongodump gzip archive)"],
};

export default function DatabaseImportTab() {
  const { database } = useOutletContext<OutletContext>();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [importResult, setImportResult] = useState<string | null>(null);

  const formats = SUPPORTED_FORMATS[database.db_type] ?? [];
  const isRunning = database.status === "running";

  const importMutation = useMutation({
    mutationFn: (file: File) => api.importDatabaseDump(database.id, file),
    onSuccess: (result) => {
      setImportResult(result.message);
      setSelectedFile(null);
      if (fileInputRef.current) {
        fileInputRef.current.value = "";
      }
      toast.success(`Dump imported successfully into ${database.name}`);
    },
    onError: (error) => {
      toast.error(
        error instanceof Error ? error.message : "Failed to import dump"
      );
    },
  });

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0] ?? null;
    setSelectedFile(file);
    setImportResult(null);
  };

  const handleImport = () => {
    if (!selectedFile) return;
    importMutation.mutate(selectedFile);
  };

  return (
    <div className="space-y-6">
      {/* Import Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Upload className="h-5 w-5" />
            Import Database Dump
          </CardTitle>
          <CardDescription>
            Upload a dump file to seed or restore the{" "}
            <strong>{database.name}</strong> database.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Database must be running warning */}
          {!isRunning && (
            <div className="rounded-lg border border-amber-200 bg-amber-50 dark:border-amber-800 dark:bg-amber-950 p-4">
              <div className="flex items-start gap-3">
                <Info className="h-5 w-5 text-amber-600 dark:text-amber-400 mt-0.5 flex-shrink-0" />
                <p className="text-sm text-amber-700 dark:text-amber-300">
                  The database must be <strong>running</strong> before you can
                  import a dump. Start the database first.
                </p>
              </div>
            </div>
          )}

          {/* Supported formats info */}
          {formats.length > 0 && (
            <div className="rounded-lg border p-4 space-y-2">
              <p className="text-sm font-medium">Supported formats for {database.db_type.toUpperCase()}:</p>
              <ul className="space-y-1">
                {formats.map((fmt) => (
                  <li key={fmt} className="text-sm text-muted-foreground flex items-center gap-2">
                    <span className="h-1.5 w-1.5 rounded-full bg-muted-foreground" />
                    {fmt}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {/* File upload area */}
          <div
            className="flex flex-col items-center justify-center gap-4 rounded-lg border-2 border-dashed p-8 cursor-pointer hover:border-primary/50 transition-colors"
            onClick={() => fileInputRef.current?.click()}
          >
            <FileUp className="h-10 w-10 text-muted-foreground" />
            <div className="text-center">
              <p className="text-sm font-medium">
                {selectedFile ? selectedFile.name : "Click to select a dump file"}
              </p>
              {selectedFile ? (
                <p className="text-xs text-muted-foreground mt-1">
                  {(selectedFile.size / 1024 / 1024).toFixed(2)} MB
                </p>
              ) : (
                <p className="text-xs text-muted-foreground mt-1">
                  SQL, dump, or archive files up to several hundred MB
                </p>
              )}
            </div>
            <input
              ref={fileInputRef}
              type="file"
              className="hidden"
              accept=".sql,.dump,.archive,.gz,.bz2,.tar"
              onChange={handleFileChange}
            />
          </div>

          {/* Import button */}
          <Button
            onClick={handleImport}
            disabled={!selectedFile || !isRunning || importMutation.isPending}
            className="w-full"
          >
            {importMutation.isPending ? (
              <>
                <Upload className="h-4 w-4 mr-2 animate-pulse" />
                Importing...
              </>
            ) : (
              <>
                <Upload className="h-4 w-4 mr-2" />
                Import Dump
              </>
            )}
          </Button>

          {/* Success message */}
          {importResult && (
            <div className="rounded-lg border border-green-200 bg-green-50 dark:border-green-800 dark:bg-green-950 p-4">
              <div className="flex items-center gap-3">
                <CheckCircle2 className="h-5 w-5 text-green-600 dark:text-green-400 flex-shrink-0" />
                <p className="text-sm text-green-700 dark:text-green-300">
                  {importResult}
                </p>
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Warning card */}
      <Card className="border-amber-200 dark:border-amber-800">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-amber-700 dark:text-amber-400 text-base">
            <Info className="h-4 w-4" />
            Before You Import
          </CardTitle>
        </CardHeader>
        <CardContent>
          <ul className="space-y-2 text-sm text-muted-foreground">
            <li className="flex items-start gap-2">
              <span className="mt-1.5 h-1.5 w-1.5 flex-shrink-0 rounded-full bg-muted-foreground" />
              Importing a dump will <strong>add data</strong> to the existing
              database. If you want a clean restore, consider dropping all tables
              or recreating the database first.
            </li>
            <li className="flex items-start gap-2">
              <span className="mt-1.5 h-1.5 w-1.5 flex-shrink-0 rounded-full bg-muted-foreground" />
              Large dumps may take several minutes to import. Do not navigate
              away until the import completes.
            </li>
            <li className="flex items-start gap-2">
              <span className="mt-1.5 h-1.5 w-1.5 flex-shrink-0 rounded-full bg-muted-foreground" />
              Take a <strong>backup</strong> first if the database contains
              important data.
            </li>
          </ul>
        </CardContent>
      </Card>
    </div>
  );
}
