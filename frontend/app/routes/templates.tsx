import { useState, useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Form, useNavigation, useNavigate } from "react-router";
import type { Route } from "./+types/templates";
import { toast } from "sonner";
import {
  Activity,
  Database,
  HardDrive,
  Code,
  BarChart,
  Network,
  Shield,
  Rocket,
  Search,
  Layers,
  X,
  Eye,
  EyeOff,
} from "lucide-react";
import { api } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type {
  ServiceTemplate,
  TemplateCategory,
  EnvSchemaEntry,
  DeployTemplateRequest,
} from "@/types/api";
import { TEMPLATE_CATEGORIES } from "@/types/api";

export function meta() {
  return [
    { title: "Templates - Rivetr" },
    { name: "description", content: "Browse and deploy service templates" },
  ];
}

export async function loader({ request }: Route.LoaderArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  const { api } = await import("@/lib/api.server");

  const token = await requireAuth(request);
  const templates = await api.getTemplates(token).catch(() => []);

  return { templates, token };
}

// Map category to icon component
function getCategoryIcon(category: TemplateCategory) {
  switch (category) {
    case "monitoring":
      return Activity;
    case "database":
      return Database;
    case "storage":
      return HardDrive;
    case "development":
      return Code;
    case "analytics":
      return BarChart;
    case "networking":
      return Network;
    case "security":
      return Shield;
    default:
      return Layers;
  }
}

function getCategoryColor(category: TemplateCategory) {
  switch (category) {
    case "monitoring":
      return "bg-blue-500/10 text-blue-500 border-blue-500/20";
    case "database":
      return "bg-purple-500/10 text-purple-500 border-purple-500/20";
    case "storage":
      return "bg-orange-500/10 text-orange-500 border-orange-500/20";
    case "development":
      return "bg-green-500/10 text-green-500 border-green-500/20";
    case "analytics":
      return "bg-yellow-500/10 text-yellow-500 border-yellow-500/20";
    case "networking":
      return "bg-cyan-500/10 text-cyan-500 border-cyan-500/20";
    case "security":
      return "bg-red-500/10 text-red-500 border-red-500/20";
    default:
      return "bg-gray-500/10 text-gray-500 border-gray-500/20";
  }
}

export default function TemplatesPage({ loaderData }: Route.ComponentProps) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [activeCategory, setActiveCategory] = useState<TemplateCategory | "all">("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedTemplate, setSelectedTemplate] = useState<ServiceTemplate | null>(null);
  const [deployName, setDeployName] = useState("");
  const [envVars, setEnvVars] = useState<Record<string, string>>({});
  const [showSecrets, setShowSecrets] = useState<Record<string, boolean>>({});

  const { data: templates = [] } = useQuery<ServiceTemplate[]>({
    queryKey: ["templates"],
    queryFn: () => api.getTemplates(undefined, loaderData.token),
    initialData: loaderData.templates,
  });

  const deployMutation = useMutation({
    mutationFn: (data: { templateId: string; request: DeployTemplateRequest }) =>
      api.deployTemplate(data.templateId, data.request, loaderData.token),
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: ["services"] });
      toast.success("Template deployed successfully");
      setSelectedTemplate(null);
      navigate(`/services/${result.service_id}`);
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to deploy template");
    },
  });

  // Filter templates by category and search
  const filteredTemplates = useMemo(() => {
    let result = templates;

    if (activeCategory !== "all") {
      result = result.filter((t) => t.category === activeCategory);
    }

    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      result = result.filter(
        (t) =>
          t.name.toLowerCase().includes(query) ||
          (t.description && t.description.toLowerCase().includes(query))
      );
    }

    return result;
  }, [templates, activeCategory, searchQuery]);

  // Group templates by category for display
  const templatesByCategory = useMemo(() => {
    const grouped: Record<string, ServiceTemplate[]> = {};
    for (const template of filteredTemplates) {
      if (!grouped[template.category]) {
        grouped[template.category] = [];
      }
      grouped[template.category].push(template);
    }
    return grouped;
  }, [filteredTemplates]);

  const openDeployDialog = (template: ServiceTemplate) => {
    setSelectedTemplate(template);
    setDeployName(template.name.toLowerCase().replace(/\s+/g, "-"));
    // Initialize env vars with defaults
    const defaults: Record<string, string> = {};
    for (const entry of template.env_schema) {
      defaults[entry.name] = entry.default || "";
    }
    setEnvVars(defaults);
    setShowSecrets({});
  };

  const handleDeploy = () => {
    if (!selectedTemplate || !deployName.trim()) return;

    deployMutation.mutate({
      templateId: selectedTemplate.id,
      request: {
        name: deployName.trim(),
        env_vars: envVars,
      },
    });
  };

  const toggleSecretVisibility = (name: string) => {
    setShowSecrets((prev) => ({ ...prev, [name]: !prev[name] }));
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-bold">Service Templates</h1>
        <p className="text-muted-foreground">
          Deploy pre-configured services with one click
        </p>
      </div>

      {/* Search and Filter */}
      <div className="flex flex-col sm:flex-row gap-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search templates..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
          {searchQuery && (
            <Button
              variant="ghost"
              size="icon"
              className="absolute right-1 top-1/2 -translate-y-1/2 h-6 w-6"
              onClick={() => setSearchQuery("")}
            >
              <X className="h-4 w-4" />
            </Button>
          )}
        </div>
        <Tabs
          value={activeCategory}
          onValueChange={(v) => setActiveCategory(v as TemplateCategory | "all")}
        >
          <TabsList className="flex-wrap h-auto">
            <TabsTrigger value="all">All</TabsTrigger>
            {TEMPLATE_CATEGORIES.map((cat) => (
              <TabsTrigger key={cat.id} value={cat.id}>
                {cat.name}
              </TabsTrigger>
            ))}
          </TabsList>
        </Tabs>
      </div>

      {/* Templates Grid */}
      {filteredTemplates.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center">
            <Layers className="mx-auto h-12 w-12 text-muted-foreground mb-4" />
            <p className="text-muted-foreground">
              {searchQuery
                ? "No templates match your search."
                : "No templates available in this category."}
            </p>
          </CardContent>
        </Card>
      ) : activeCategory === "all" ? (
        // Grouped by category when showing all
        <div className="space-y-8">
          {Object.entries(templatesByCategory).map(([category, categoryTemplates]) => {
            const categoryInfo = TEMPLATE_CATEGORIES.find((c) => c.id === category);
            const Icon = getCategoryIcon(category as TemplateCategory);
            return (
              <div key={category}>
                <div className="flex items-center gap-2 mb-4">
                  <Icon className="h-5 w-5" />
                  <h2 className="text-xl font-semibold">{categoryInfo?.name || category}</h2>
                  <Badge variant="secondary">{categoryTemplates.length}</Badge>
                </div>
                <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                  {categoryTemplates.map((template) => (
                    <TemplateCard
                      key={template.id}
                      template={template}
                      onDeploy={() => openDeployDialog(template)}
                    />
                  ))}
                </div>
              </div>
            );
          })}
        </div>
      ) : (
        // Flat grid when filtered by category
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {filteredTemplates.map((template) => (
            <TemplateCard
              key={template.id}
              template={template}
              onDeploy={() => openDeployDialog(template)}
            />
          ))}
        </div>
      )}

      {/* Deploy Dialog */}
      <Dialog
        open={!!selectedTemplate}
        onOpenChange={(open) => !open && setSelectedTemplate(null)}
      >
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>Deploy {selectedTemplate?.name}</DialogTitle>
            <DialogDescription>
              {selectedTemplate?.description || "Configure and deploy this template."}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="deploy-name">Service Name</Label>
              <Input
                id="deploy-name"
                value={deployName}
                onChange={(e) => setDeployName(e.target.value)}
                placeholder="my-service"
                pattern="[a-zA-Z0-9-]+"
                title="Only alphanumeric characters and hyphens"
                required
              />
              <p className="text-xs text-muted-foreground">
                Only letters, numbers, and hyphens allowed
              </p>
            </div>

            {selectedTemplate?.env_schema && selectedTemplate.env_schema.length > 0 && (
              <div className="space-y-4">
                <Label>Configuration</Label>
                {selectedTemplate.env_schema.map((entry) => (
                  <div key={entry.name} className="space-y-1">
                    <Label htmlFor={`env-${entry.name}`} className="text-sm">
                      {entry.label}
                      {entry.required && <span className="text-destructive ml-1">*</span>}
                    </Label>
                    <div className="relative">
                      <Input
                        id={`env-${entry.name}`}
                        type={entry.secret && !showSecrets[entry.name] ? "password" : "text"}
                        value={envVars[entry.name] || ""}
                        onChange={(e) =>
                          setEnvVars((prev) => ({ ...prev, [entry.name]: e.target.value }))
                        }
                        placeholder={entry.default || `Enter ${entry.label.toLowerCase()}`}
                        required={entry.required}
                        className={entry.secret ? "pr-10" : ""}
                      />
                      {entry.secret && (
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          className="absolute right-0 top-0 h-full px-3"
                          onClick={() => toggleSecretVisibility(entry.name)}
                        >
                          {showSecrets[entry.name] ? (
                            <EyeOff className="h-4 w-4" />
                          ) : (
                            <Eye className="h-4 w-4" />
                          )}
                        </Button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setSelectedTemplate(null)}>
              Cancel
            </Button>
            <Button
              onClick={handleDeploy}
              disabled={deployMutation.isPending || !deployName.trim()}
            >
              <Rocket className="mr-2 h-4 w-4" />
              {deployMutation.isPending ? "Deploying..." : "Deploy"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

// Template Card Component
function TemplateCard({
  template,
  onDeploy,
}: {
  template: ServiceTemplate;
  onDeploy: () => void;
}) {
  const Icon = getCategoryIcon(template.category);
  const colorClass = getCategoryColor(template.category);

  return (
    <Card className="flex flex-col hover:shadow-md transition-shadow">
      <CardHeader className="pb-3">
        <div className="flex items-start justify-between">
          <div className={`p-2 rounded-lg ${colorClass}`}>
            <Icon className="h-5 w-5" />
          </div>
          {template.is_builtin && (
            <Badge variant="secondary" className="text-xs">
              Built-in
            </Badge>
          )}
        </div>
        <CardTitle className="text-lg mt-3">{template.name}</CardTitle>
        {template.description && (
          <CardDescription className="line-clamp-2">{template.description}</CardDescription>
        )}
      </CardHeader>
      <CardContent className="flex-1 flex flex-col justify-end">
        <div className="flex items-center justify-between">
          <Badge variant="outline" className="capitalize">
            {template.category}
          </Badge>
          <Button size="sm" onClick={onDeploy}>
            <Rocket className="mr-2 h-3 w-3" />
            Deploy
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
