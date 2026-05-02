import { useState, useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate, Link } from "react-router";
import { toast } from "sonner";
import { communityTemplatesApi } from "@/lib/api/community-templates";
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
  Lightbulb,
  Send,
} from "lucide-react";
import { api } from "@/lib/api";
import { servicesApi } from "@/lib/api/services";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
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

const SUGGEST_CATEGORIES = [
  "monitoring",
  "database",
  "storage",
  "development",
  "analytics",
  "networking",
  "security",
  "ai",
  "automation",
  "cms",
  "communication",
  "other",
];

interface SuggestForm {
  name: string;
  description: string;
  docker_image: string;
  category: string;
  website_url: string;
  notes: string;
}

export default function TemplatesPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [activeCategory, setActiveCategory] = useState<TemplateCategory | "all">("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedTemplate, setSelectedTemplate] = useState<ServiceTemplate | null>(null);
  const [deployName, setDeployName] = useState("");
  const [envVars, setEnvVars] = useState<Record<string, string>>({});
  const [showSecrets, setShowSecrets] = useState<Record<string, boolean>>({});

  // Submit Community Template state
  const [showSubmitDialog, setShowSubmitDialog] = useState(false);
  const [submitForm, setSubmitForm] = useState({
    name: "",
    description: "",
    category: "other",
    icon: "",
    compose_content: "",
  });

  const submitTemplateMutation = useMutation({
    mutationFn: () =>
      communityTemplatesApi.submit({
        name: submitForm.name.trim(),
        description: submitForm.description.trim(),
        category: submitForm.category,
        icon: submitForm.icon.trim() || undefined,
        compose_content: submitForm.compose_content.trim(),
      }),
    onSuccess: () => {
      toast.success("Template submitted for admin review!");
      setShowSubmitDialog(false);
      setSubmitForm({ name: "", description: "", category: "other", icon: "", compose_content: "" });
    },
    onError: (err: Error) => {
      toast.error(err.message || "Failed to submit template");
    },
  });

  // Suggest Template state
  const [showSuggestDialog, setShowSuggestDialog] = useState(false);
  const [suggestForm, setSuggestForm] = useState<SuggestForm>({
    name: "",
    description: "",
    docker_image: "",
    category: "other",
    website_url: "",
    notes: "",
  });

  const { data: templates = [], isLoading } = useQuery<ServiceTemplate[]>({
    queryKey: ["templates"],
    queryFn: () => api.getTemplates(),
  });

  const suggestMutation = useMutation({
    mutationFn: () =>
      servicesApi.suggestTemplate({
        name: suggestForm.name.trim(),
        description: suggestForm.description.trim(),
        docker_image: suggestForm.docker_image.trim(),
        category: suggestForm.category,
        website_url: suggestForm.website_url.trim() || undefined,
        notes: suggestForm.notes.trim() || undefined,
      }),
    onSuccess: () => {
      toast.success("Template suggestion submitted! We'll review it soon.");
      setShowSuggestDialog(false);
      setSuggestForm({
        name: "",
        description: "",
        docker_image: "",
        category: "other",
        website_url: "",
        notes: "",
      });
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : "Failed to submit suggestion");
    },
  });

  const handleSuggestSubmit = () => {
    if (!suggestForm.name.trim() || !suggestForm.docker_image.trim()) {
      toast.error("Name and Docker image are required");
      return;
    }
    suggestMutation.mutate();
  };

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
        <div className="flex gap-2">
          <Link to="/templates/submissions">
            <Button variant="outline" size="sm">
              My Submissions
            </Button>
          </Link>
          <Button variant="outline" onClick={() => setShowSubmitDialog(true)}>
            <Send className="mr-2 h-4 w-4" />
            Submit Template
          </Button>
          <Button variant="outline" onClick={() => setShowSuggestDialog(true)}>
            <Lightbulb className="mr-2 h-4 w-4" />
            Suggest a Template
          </Button>
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
        // light (B18 — was rendering ~1300 cards), cap each category to a
        // small preview when the user hasn't searched. Users click into a
        // specific category tab to see the full list.
        <div className="space-y-8">
          {Object.entries(templatesByCategory).map(([category, categoryTemplates]) => {
            const categoryInfo = TEMPLATE_CATEGORIES.find((c) => c.id === category);
            const Icon = getCategoryIcon(category as TemplateCategory);
            const PREVIEW_LIMIT = 6;
            const isPreviewing =
              !searchQuery.trim() && categoryTemplates.length > PREVIEW_LIMIT;
            const visibleTemplates = isPreviewing
              ? categoryTemplates.slice(0, PREVIEW_LIMIT)
              : categoryTemplates;
            return (
              <div key={category}>
                <div className="flex items-center gap-2 mb-4">
                  <Icon className="h-5 w-5" />
                  <h2 className="text-xl font-semibold">{categoryInfo?.name || category}</h2>
                  <Badge variant="secondary">{categoryTemplates.length}</Badge>
                  {isPreviewing && (
                    <Button
                      variant="link"
                      size="sm"
                      className="ml-auto h-auto p-0"
                      onClick={() => setActiveCategory(category as TemplateCategory)}
                    >
                      View all {categoryTemplates.length}
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

      {/* Suggest Template Dialog */}
      <Dialog open={showSuggestDialog} onOpenChange={setShowSuggestDialog}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>Suggest a Template</DialogTitle>
            <DialogDescription>
              Know a great Docker image that should be a template? Submit it for review and we'll add it to the library.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="space-y-2">
              <Label htmlFor="suggest-name">
                Template Name <span className="text-destructive">*</span>
              </Label>
              <Input
                id="suggest-name"
                value={suggestForm.name}
                onChange={(e) => setSuggestForm((f) => ({ ...f, name: e.target.value }))}
                placeholder="e.g. Minio, Grafana, n8n"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="suggest-docker-image">
                Docker Image <span className="text-destructive">*</span>
              </Label>
              <Input
                id="suggest-docker-image"
                value={suggestForm.docker_image}
                onChange={(e) => setSuggestForm((f) => ({ ...f, docker_image: e.target.value }))}
                placeholder="e.g. minio/minio:latest"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="suggest-description">Description</Label>
              <Textarea
                id="suggest-description"
                value={suggestForm.description}
                onChange={(e) => setSuggestForm((f) => ({ ...f, description: e.target.value }))}
                placeholder="What does this service do?"
                rows={2}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="suggest-category">Category</Label>
              <Select
                value={suggestForm.category}
                onValueChange={(v) => setSuggestForm((f) => ({ ...f, category: v }))}
              >
                <SelectTrigger id="suggest-category">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {SUGGEST_CATEGORIES.map((cat) => (
                    <SelectItem key={cat} value={cat} className="capitalize">
                      {cat}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="suggest-website">Website URL</Label>
              <Input
                id="suggest-website"
                value={suggestForm.website_url}
                onChange={(e) => setSuggestForm((f) => ({ ...f, website_url: e.target.value }))}
                placeholder="https://example.com"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="suggest-notes">Additional Notes</Label>
              <Textarea
                id="suggest-notes"
                value={suggestForm.notes}
                onChange={(e) => setSuggestForm((f) => ({ ...f, notes: e.target.value }))}
                placeholder="Any special configuration, ports, or environment variables we should know about?"
                rows={3}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowSuggestDialog(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleSuggestSubmit}
              disabled={suggestMutation.isPending || !suggestForm.name.trim() || !suggestForm.docker_image.trim()}
            >
              {suggestMutation.isPending ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Lightbulb className="mr-2 h-4 w-4" />
              )}
              {suggestMutation.isPending ? "Submitting..." : "Submit Suggestion"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Submit Community Template Dialog */}
      <Dialog open={showSubmitDialog} onOpenChange={setShowSubmitDialog}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Submit a Community Template</DialogTitle>
            <DialogDescription>
              Share a Docker Compose template with the community. Admins will review and approve it.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label>Name *</Label>
                <Input
                  placeholder="e.g. My App"
                  value={submitForm.name}
                  onChange={(e) => setSubmitForm((f) => ({ ...f, name: e.target.value }))}
                />
              </div>
              <div className="space-y-2">
                <Label>Category *</Label>
                <Select
                  value={submitForm.category}
                  onValueChange={(v) => setSubmitForm((f) => ({ ...f, category: v }))}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {SUGGEST_CATEGORIES.map((cat) => (
                      <SelectItem key={cat} value={cat} className="capitalize">
                        {cat}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
            <div className="space-y-2">
              <Label>Description *</Label>
              <Input
                placeholder="Brief description of what this template does"
                value={submitForm.description}
                onChange={(e) => setSubmitForm((f) => ({ ...f, description: e.target.value }))}
              />
            </div>
            <div className="space-y-2">
              <Label>Icon (emoji, optional)</Label>
              <Input
                placeholder="e.g. 🚀"
                value={submitForm.icon}
                onChange={(e) => setSubmitForm((f) => ({ ...f, icon: e.target.value }))}
                className="w-24"
              />
            </div>
            <div className="space-y-2">
              <Label>Docker Compose Content *</Label>
              <Textarea
                placeholder={"services:\n  myapp:\n    image: myapp:latest\n    ports:\n      - '8080:8080'"}
                value={submitForm.compose_content}
                onChange={(e) => setSubmitForm((f) => ({ ...f, compose_content: e.target.value }))}
                rows={8}
                className="font-mono text-sm"
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowSubmitDialog(false)}>
              Cancel
            </Button>
            <Button
              disabled={
                submitTemplateMutation.isPending ||
                !submitForm.name.trim() ||
                !submitForm.description.trim() ||
                !submitForm.compose_content.trim()
              }
              onClick={() => submitTemplateMutation.mutate()}
            >
              {submitTemplateMutation.isPending ? (
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              ) : (
                <Send className="h-4 w-4 mr-2" />
              )}
              Submit for Review
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

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
