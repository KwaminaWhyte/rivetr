import { useOutletContext } from "react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Check, Download } from "lucide-react";
import { api } from "@/lib/api";
import type { ManagedDatabase } from "@/types/api";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

export function meta() {
  return [
    { title: "Database Extensions - Rivetr" },
    {
      name: "description",
      content: "Manage PostgreSQL extensions for this database",
    },
  ];
}

interface OutletContext {
  database: ManagedDatabase;
}

interface Extension {
  name: string;
  description: string;
  category: string;
}

const CURATED_EXTENSIONS: Extension[] = [
  {
    name: "pgvector",
    description: "Vector similarity search for AI/ML workloads",
    category: "AI / ML",
  },
  {
    name: "apache_age",
    description: "Graph database extension (Apache AGE)",
    category: "Graph",
  },
  {
    name: "postgis",
    description: "Geographic and spatial data support",
    category: "Geospatial",
  },
  {
    name: "pg_trgm",
    description: "Fuzzy text search using trigrams",
    category: "Search",
  },
  {
    name: "uuid-ossp",
    description: "UUID generation functions",
    category: "Utilities",
  },
  {
    name: "hstore",
    description: "Key-value store as a column type",
    category: "Data Types",
  },
  {
    name: "pg_stat_statements",
    description: "Query performance statistics",
    category: "Monitoring",
  },
  {
    name: "timescaledb",
    description: "Time-series data extension",
    category: "Time-Series",
  },
  {
    name: "citext",
    description: "Case-insensitive text data type",
    category: "Data Types",
  },
  {
    name: "btree_gin",
    description: "GIN index support for common data types",
    category: "Indexing",
  },
  {
    name: "btree_gist",
    description: "GiST index support for common data types",
    category: "Indexing",
  },
  {
    name: "unaccent",
    description: "Text search dictionary that removes accents",
    category: "Search",
  },
];

export default function DatabaseExtensionsTab() {
  const { database } = useOutletContext<OutletContext>();
  const queryClient = useQueryClient();

  const isPostgres = database.db_type === "postgres";
  const isRunning = database.status === "running";

  const { data: extensionsData, isLoading } = useQuery({
    queryKey: ["database-extensions", database.id],
    queryFn: () => api.listDatabaseExtensions(database.id),
    enabled: isPostgres && isRunning,
    refetchInterval: false,
  });

  const installedNames = new Set(
    extensionsData?.extensions.map((e) => e.name) ?? []
  );

  const installMutation = useMutation({
    mutationFn: (extension: string) =>
      api.installDatabaseExtension(database.id, extension),
    onSuccess: (_data, extension) => {
      toast.success(`Extension "${extension}" installed`);
      queryClient.invalidateQueries({
        queryKey: ["database-extensions", database.id],
      });
    },
    onError: (err: Error, extension) => {
      toast.error(`Failed to install "${extension}": ${err.message}`);
    },
  });

  if (!isPostgres) {
    return (
      <Card>
        <CardContent className="py-8 text-center text-muted-foreground">
          Extensions are only available for PostgreSQL databases.
        </CardContent>
      </Card>
    );
  }

  if (!isRunning) {
    return (
      <Card>
        <CardContent className="py-8 text-center text-muted-foreground">
          The database must be running to manage extensions. Start the database
          first.
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
      {/* Installed Extensions */}
      <Card>
        <CardHeader>
          <CardTitle>Installed Extensions</CardTitle>
          <CardDescription>
            Extensions currently active in this database
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex flex-wrap gap-2">
              {[1, 2, 3].map((i) => (
                <div
                  key={i}
                  className="h-6 w-24 bg-muted animate-pulse rounded"
                />
              ))}
            </div>
          ) : installedNames.size === 0 ? (
            <p className="text-sm text-muted-foreground">
              No extensions installed yet.
            </p>
          ) : (
            <div className="flex flex-wrap gap-2">
              {Array.from(installedNames).map((name) => (
                <Badge key={name} variant="secondary" className="font-mono text-xs">
                  {name}
                </Badge>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Available Extensions */}
      <Card>
        <CardHeader>
          <CardTitle>Available Extensions</CardTitle>
          <CardDescription>
            Popular PostgreSQL extensions you can install with one click
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {CURATED_EXTENSIONS.map((ext) => {
              const installed = installedNames.has(ext.name);
              const isInstalling =
                installMutation.isPending &&
                installMutation.variables === ext.name;

              return (
                <div
                  key={ext.name}
                  className="flex items-start justify-between rounded-lg border p-3 gap-3"
                >
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2 flex-wrap">
                      <code className="text-sm font-medium">{ext.name}</code>
                      <Badge
                        variant="outline"
                        className="text-xs shrink-0"
                      >
                        {ext.category}
                      </Badge>
                    </div>
                    <p className="text-xs text-muted-foreground mt-1">
                      {ext.description}
                    </p>
                  </div>
                  <div className="shrink-0">
                    {installed ? (
                      <Badge className="bg-green-500 hover:bg-green-600 gap-1">
                        <Check className="h-3 w-3" />
                        Installed
                      </Badge>
                    ) : (
                      <Button
                        size="sm"
                        variant="outline"
                        className="h-7 gap-1"
                        disabled={isInstalling || installMutation.isPending}
                        onClick={() => installMutation.mutate(ext.name)}
                      >
                        <Download className="h-3 w-3" />
                        {isInstalling ? "Installing..." : "Install"}
                      </Button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
