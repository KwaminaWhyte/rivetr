import { useState, useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router";
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
  Loader2,
  Brain,
  Zap,
  FileText,
  MessageCircle,
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
  DeployTemplateRequest,
} from "@/types/api";
import { TEMPLATE_CATEGORIES } from "@/types/api";

export function meta() {
  return [
    { title: "Templates - Rivetr" },
    { name: "description", content: "Browse and deploy service templates" },
  ];
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
    case "ai":
      return Brain;
    case "automation":
      return Zap;
    case "cms":
      return FileText;
    case "communication":
      return MessageCircle;
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
    case "ai":
      return "bg-violet-500/10 text-violet-500 border-violet-500/20";
    case "automation":
      return "bg-amber-500/10 text-amber-500 border-amber-500/20";
    case "cms":
      return "bg-emerald-500/10 text-emerald-500 border-emerald-500/20";
    case "communication":
      return "bg-sky-500/10 text-sky-500 border-sky-500/20";
    default:
      return "bg-gray-500/10 text-gray-500 border-gray-500/20";
  }
}

export default function TemplatesPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [activeCategory, setActiveCategory] = useState<TemplateCategory | "all">("all");
  const [searchQuery, setSearchQuery] = useState("");
  // Track which category sections the user has expanded inline (overrides the
  // PREVIEW_LIMIT cap on the "all" view so users can see every card without
  // leaving the grouped layout). U5: also gives us a target to scrollIntoView.
  const [expandedCategories, setExpandedCategories] = useState<Set<string>>(new Set());
  const [selectedTemplate, setSelectedTemplate] = useState<ServiceTemplate | null>(null);
  const [deployName, setDeployName] = useState("");
  const [envVars, setEnvVars] = useState<Record<string, string>>({});
  const [showSecrets, setShowSecrets] = useState<Record<string, boolean>>({});

  const { data: templates = [], isLoading } = useQuery<ServiceTemplate[]>({
    queryKey: ["templates"],
    queryFn: () => api.getTemplates(),
  });

  const deployMutation = useMutation({
    mutationFn: (data: { templateId: string; request: DeployTemplateRequest }) =>
      api.deployTemplate(data.templateId, data.request),
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
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-3xl font-bold">Service Templates</h1>
          <p className="text-muted-foreground">
            Deploy pre-configured services with one click
          </p>
        </div>
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
          onValueChange={(v) => {
            const next = v as TemplateCategory | "all";
            setActiveCategory(next);
            // U5: after the filtered grid mounts, scroll to the top of the
            // page so users see the category heading instead of being left
            // wherever the previous scroll position was.
            if (next !== "all") {
              requestAnimationFrame(() => {
                window.scrollTo({ top: 0, behavior: "smooth" });
              });
            }
          }}
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
      {isLoading ? (
        <Card>
          <CardContent className="py-12 text-center">
            <Loader2 className="mx-auto h-12 w-12 text-muted-foreground mb-4 animate-spin" />
            <p className="text-muted-foreground">Loading templates...</p>
          </CardContent>
        </Card>
      ) : filteredTemplates.length === 0 ? (
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
        // Grouped by category when showing all. To keep the initial paint
        // light (B18, was rendering ~1300 cards), cap each category to a
        // small preview when the user hasn't searched. Users click into a
        // specific category tab to see the full list.
        <div className="space-y-8">
          {Object.entries(templatesByCategory).map(([category, categoryTemplates]) => {
            const categoryInfo = TEMPLATE_CATEGORIES.find((c) => c.id === category);
            const Icon = getCategoryIcon(category as TemplateCategory);
            const PREVIEW_LIMIT = 6;
            const isExpanded = expandedCategories.has(category);
            const isPreviewing =
              !searchQuery.trim() &&
              !isExpanded &&
              categoryTemplates.length > PREVIEW_LIMIT;
            const visibleTemplates = isPreviewing
              ? categoryTemplates.slice(0, PREVIEW_LIMIT)
              : categoryTemplates;
            const anchorId = `category-${category.toLowerCase()}`;
            return (
              <div key={category} id={anchorId} className="scroll-mt-20">
                <div className="flex items-center gap-2 mb-4">
                  <Icon className="h-5 w-5" />
                  <h2 className="text-xl font-semibold">{categoryInfo?.name || category}</h2>
                  <Badge variant="secondary">{categoryTemplates.length}</Badge>
                  {isPreviewing && (
                    <Button
                      variant="link"
                      size="sm"
                      className="ml-auto h-auto p-0"
                      onClick={() => {
                        setExpandedCategories((prev) => {
                          const next = new Set(prev);
                          next.add(category);
                          return next;
                        });
                        // Defer to next frame so the newly rendered cards
                        // don't push the heading off-screen before we scroll.
                        requestAnimationFrame(() => {
                          document
                            .getElementById(anchorId)
                            ?.scrollIntoView({ behavior: "smooth", block: "start" });
                        });
                      }}
                    >
                      View all {categoryTemplates.length}
                    </Button>
                  )}
                  {isExpanded && categoryTemplates.length > PREVIEW_LIMIT && (
                    <Button
                      variant="link"
                      size="sm"
                      className="ml-auto h-auto p-0"
                      onClick={() => {
                        setExpandedCategories((prev) => {
                          const next = new Set(prev);
                          next.delete(category);
                          return next;
                        });
                      }}
                    >
                      Show less
                    </Button>
                  )}
                </div>
                <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                  {visibleTemplates.map((template) => (
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
