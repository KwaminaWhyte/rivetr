import { useState, useMemo } from "react";
import { Link } from "react-router";
import { useQueryClient, useMutation, useQuery } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  AlertCircle,
  Eye,
  EyeOff,
  Layers,
  Play,
  Plus,
  Rocket,
  Search,
  Square,
  Trash2,
} from "lucide-react";
import { api } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { ProjectWithApps, Service, ServiceTemplate } from "@/types/api";

interface ServicesTabProps {
  project: ProjectWithApps;
  projectId: string;
}

function ServiceStatusBadge({ status }: { status: string }) {
  switch (status) {
    case "running":
      return <Badge className="bg-green-500 hover:bg-green-600">Running</Badge>;
    case "stopped":
      return <Badge variant="secondary">Stopped</Badge>;
    case "pending":
      return <Badge variant="outline">Pending</Badge>;
    case "failed":
      return <Badge variant="destructive">Failed</Badge>;
    default:
      return <Badge variant="outline">{status}</Badge>;
  }
}

export function ServicesTab({ project, projectId }: ServicesTabProps) {
  const queryClient = useQueryClient();
  const [isCreateServiceDialogOpen, setIsCreateServiceDialogOpen] = useState(false);
  const [isDeleteServiceDialogOpen, setIsDeleteServiceDialogOpen] = useState(false);
  const [selectedService, setSelectedService] = useState<Service | null>(null);
  const [isTemplatesModalOpen, setIsTemplatesModalOpen] = useState(false);
  const [templateSearch, setTemplateSearch] = useState("");
  const [selectedCategory, setSelectedCategory] = useState<string>("all");
  const [selectedTemplate, setSelectedTemplate] = useState<ServiceTemplate | null>(null);
  const [templateServiceName, setTemplateServiceName] = useState("");
  const [templateEnvVars, setTemplateEnvVars] = useState<Record<string, string>>({});
  const [showTemplateSecrets, setShowTemplateSecrets] = useState<Record<string, boolean>>({});

  // Form state
  const [serviceName, setServiceName] = useState("");
  const [composeContent, setComposeContent] = useState(`version: "3.8"
services:
  app:
    image: nginx:alpine
    ports:
      - "80"
`);

  const { data: templates = [] } = useQuery<ServiceTemplate[]>({
    queryKey: ["service-templates"],
    queryFn: () => api.getTemplates(),
  });

  const categories = useMemo(() => {
    const cats = new Set(templates.map((t) => t.category));
    return ["all", ...Array.from(cats).sort()];
  }, [templates]);

  const filteredTemplates = useMemo(() => {
    return templates.filter((t) => {
      const matchesSearch =
        !templateSearch ||
        t.name.toLowerCase().includes(templateSearch.toLowerCase()) ||
        (t.description && t.description.toLowerCase().includes(templateSearch.toLowerCase()));
      const matchesCategory =
        selectedCategory === "all" || t.category === selectedCategory;
      return matchesSearch && matchesCategory;
    });
  }, [templates, templateSearch, selectedCategory]);

  const createServiceMutation = useMutation({
    mutationFn: async () => {
      if (!serviceName.trim()) {
        throw new Error("Service name is required");
      }
      if (!composeContent.trim()) {
        throw new Error("Docker Compose content is required");
      }
      return api.createService({
        name: serviceName.trim(),
        compose_content: composeContent.trim(),
        project_id: projectId,
      });
    },
    onSuccess: () => {
      toast.success("Service created");
      setIsCreateServiceDialogOpen(false);
      setServiceName("");
      setComposeContent(`version: "3.8"
services:
  app:
    image: nginx:alpine
    ports:
      - "80"
`);
      queryClient.invalidateQueries({ queryKey: ["project", projectId] });
      queryClient.invalidateQueries({ queryKey: ["services"] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const deleteServiceMutation = useMutation({
    mutationFn: (serviceId: string) => api.deleteService(serviceId),
    onSuccess: () => {
      toast.success("Service deleted");
      setIsDeleteServiceDialogOpen(false);
      setSelectedService(null);
      queryClient.invalidateQueries({ queryKey: ["project", projectId] });
      queryClient.invalidateQueries({ queryKey: ["services"] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const startServiceMutation = useMutation({
    mutationFn: (serviceId: string) => api.startService(serviceId),
    onSuccess: () => {
      toast.success("Service starting");
      queryClient.invalidateQueries({ queryKey: ["project", projectId] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const stopServiceMutation = useMutation({
    mutationFn: (serviceId: string) => api.stopService(serviceId),
    onSuccess: () => {
      toast.success("Service stopped");
      queryClient.invalidateQueries({ queryKey: ["project", projectId] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const deployTemplateMutation = useMutation({
    mutationFn: async () => {
      if (!selectedTemplate) {
        throw new Error("No template selected");
      }
      if (!templateServiceName.trim()) {
        throw new Error("Service name is required");
      }
      return api.deployTemplate(selectedTemplate.id, {
        name: templateServiceName.trim(),
        project_id: projectId,
        env_vars: templateEnvVars,
      });
    },
    onSuccess: () => {
      toast.success("Service deployed from template");
      setIsTemplatesModalOpen(false);
      setSelectedTemplate(null);
      setTemplateServiceName("");
      setTemplateEnvVars({});
      setShowTemplateSecrets({});
      queryClient.invalidateQueries({ queryKey: ["project", projectId] });
      queryClient.invalidateQueries({ queryKey: ["services"] });
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  return (
    <>
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Services</CardTitle>
          <div className="flex gap-2">
            <Button variant="outline" onClick={() => setIsTemplatesModalOpen(true)}>
              <Rocket className="mr-2 h-4 w-4" />
              Deploy Template
            </Button>
            <Button onClick={() => setIsCreateServiceDialogOpen(true)}>
              <Layers className="mr-2 h-4 w-4" />
              Custom Service
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {!project.services || project.services.length === 0 ? (
            <div className="py-8 text-center">
              <Layers className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
              <p className="text-muted-foreground mb-4">
                No services in this project yet.
              </p>
              <div className="flex justify-center gap-2">
                <Button variant="outline" onClick={() => setIsTemplatesModalOpen(true)}>
                  <Rocket className="mr-2 h-4 w-4" />
                  Deploy Template
                </Button>
                <Button onClick={() => setIsCreateServiceDialogOpen(true)}>
                  <Layers className="mr-2 h-4 w-4" />
                  Custom Service
                </Button>
              </div>
            </div>
          ) : (
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              {project.services.map((service) => (
                <Card
                  key={service.id}
                  className="group relative hover:shadow-md transition-shadow"
                >
                  <Link to={`/services/${service.id}`} className="absolute inset-0 z-0" />
                  <CardHeader className="pb-2">
                    <div className="flex items-start justify-between">
                      <div className="space-y-1">
                        <div className="flex items-center gap-2">
                          <CardTitle className="text-base font-semibold">
                            {service.name}
                          </CardTitle>
                          {service.status === "failed" && service.error_message && (
                            <TooltipProvider>
                              <Tooltip>
                                <TooltipTrigger>
                                  <AlertCircle className="h-4 w-4 text-destructive" />
                                </TooltipTrigger>
                                <TooltipContent className="max-w-xs">
                                  <p className="text-sm">{service.error_message}</p>
                                </TooltipContent>
                              </Tooltip>
                            </TooltipProvider>
                          )}
                        </div>
                        <ServiceStatusBadge status={service.status} />
                      </div>
                      <div className="flex items-center gap-1 relative z-10 opacity-0 group-hover:opacity-100 transition-opacity">
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-7 w-7 text-destructive"
                          title="Delete Service"
                          onClick={(e) => {
                            e.preventDefault();
                            setSelectedService(service);
                            setIsDeleteServiceDialogOpen(true);
                          }}
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </Button>
                      </div>
                    </div>
                  </CardHeader>
                  <CardContent className="pt-0 pb-4">
                    <div className="flex items-center justify-end text-sm">
                      <div className="relative z-10 flex items-center gap-1">
                        {service.status === "stopped" && (
                          <Button
                            variant="outline"
                            size="sm"
                            className="h-7 px-2"
                            disabled={startServiceMutation.isPending}
                            onClick={(e) => {
                              e.preventDefault();
                              startServiceMutation.mutate(service.id);
                            }}
                          >
                            <Play className="h-3 w-3 mr-1" />
                            Start
                          </Button>
                        )}
                        {service.status === "running" && (
                          <Button
                            variant="outline"
                            size="sm"
                            className="h-7 px-2"
                            disabled={stopServiceMutation.isPending}
                            onClick={(e) => {
                              e.preventDefault();
                              stopServiceMutation.mutate(service.id);
                            }}
                          >
                            <Square className="h-3 w-3 mr-1" />
                            Stop
                          </Button>
                        )}
                      </div>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Create Service Dialog */}
      <Dialog open={isCreateServiceDialogOpen} onOpenChange={setIsCreateServiceDialogOpen}>
        <DialogContent className="max-w-2xl">
          <form
            onSubmit={(e) => {
              e.preventDefault();
              createServiceMutation.mutate();
            }}
          >
            <DialogHeader>
              <DialogTitle>Create Docker Compose Service</DialogTitle>
              <DialogDescription>
                Deploy a multi-container application using Docker Compose.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label htmlFor="service-name">Service Name</Label>
                <Input
                  id="service-name"
                  value={serviceName}
                  onChange={(e) => setServiceName(e.target.value)}
                  placeholder="my-service"
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="compose-content">Docker Compose Content</Label>
                <Textarea
                  id="compose-content"
                  value={composeContent}
                  onChange={(e) => setComposeContent(e.target.value)}
                  placeholder="Paste your docker-compose.yml content..."
                  className="font-mono text-sm"
                  rows={12}
                  required
                />
              </div>
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => setIsCreateServiceDialogOpen(false)}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={createServiceMutation.isPending}>
                {createServiceMutation.isPending ? "Creating..." : "Create Service"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Delete Service Dialog */}
      <Dialog open={isDeleteServiceDialogOpen} onOpenChange={setIsDeleteServiceDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Service</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete "{selectedService?.name}"? This will stop all
              containers and remove all data. This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setIsDeleteServiceDialogOpen(false);
                setSelectedService(null);
              }}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              disabled={deleteServiceMutation.isPending}
              onClick={() => {
                if (selectedService) {
                  deleteServiceMutation.mutate(selectedService.id);
                }
              }}
            >
              {deleteServiceMutation.isPending ? "Deleting..." : "Delete Service"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Templates Modal */}
      <Dialog
        open={isTemplatesModalOpen}
        onOpenChange={(open) => {
          setIsTemplatesModalOpen(open);
          if (!open) {
            setSelectedTemplate(null);
            setTemplateServiceName("");
            setTemplateSearch("");
            setSelectedCategory("all");
            setTemplateEnvVars({});
            setShowTemplateSecrets({});
          }
        }}
      >
        <DialogContent className="min-w-4xl max-h-[85vh]">
          {!selectedTemplate ? (
            <>
              <DialogHeader>
                <DialogTitle>Deploy Service from Template</DialogTitle>
                <DialogDescription>
                  Choose a pre-configured service template to deploy to this project.
                </DialogDescription>
              </DialogHeader>
              <div className="space-y-4">
                <div className="flex items-center gap-4">
                  <div className="relative flex-1">
                    <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                    <Input
                      placeholder="Search templates..."
                      value={templateSearch}
                      onChange={(e) => setTemplateSearch(e.target.value)}
                      className="pl-9"
                    />
                  </div>
                  <Tabs value={selectedCategory} onValueChange={setSelectedCategory}>
                    <TabsList>
                      {categories.slice(0, 5).map((cat) => (
                        <TabsTrigger key={cat} value={cat} className="capitalize">
                          {cat}
                        </TabsTrigger>
                      ))}
                    </TabsList>
                  </Tabs>
                </div>

                <ScrollArea className="h-[400px] pr-4">
                  {filteredTemplates.length === 0 ? (
                    <div className="py-8 text-center text-muted-foreground">
                      No templates found matching your search.
                    </div>
                  ) : (
                    <div className="grid gap-3 sm:grid-cols-2">
                      {filteredTemplates.map((template) => (
                        <button
                          key={template.id}
                          type="button"
                          className="p-4 border rounded-lg text-left hover:border-primary hover:bg-muted/50 transition-colors"
                          onClick={() => {
                            setSelectedTemplate(template);
                            setTemplateServiceName(
                              template.name.toLowerCase().replace(/[^a-z0-9]/g, "-")
                            );
                            const defaults: Record<string, string> = {};
                            defaults["PORT"] = "8080";
                            if (template.env_schema) {
                              for (const entry of template.env_schema) {
                                defaults[entry.name] = entry.default || "";
                              }
                            }
                            setTemplateEnvVars(defaults);
                            setShowTemplateSecrets({});
                          }}
                        >
                          <div className="flex items-start justify-between gap-2">
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center gap-2">
                                <span className="font-semibold truncate">{template.name}</span>
                                {template.is_builtin && (
                                  <Badge variant="secondary" className="text-xs">
                                    Built-in
                                  </Badge>
                                )}
                              </div>
                              <p className="text-sm text-muted-foreground line-clamp-2 mt-1">
                                {template.description}
                              </p>
                            </div>
                          </div>
                          <div className="flex items-center justify-between mt-3">
                            <Badge variant="outline" className="text-xs capitalize">
                              {template.category}
                            </Badge>
                            <Rocket className="h-4 w-4 text-muted-foreground" />
                          </div>
                        </button>
                      ))}
                    </div>
                  )}
                </ScrollArea>
              </div>
              <DialogFooter>
                <Button variant="outline" onClick={() => setIsTemplatesModalOpen(false)}>
                  Cancel
                </Button>
              </DialogFooter>
            </>
          ) : (
            <form
              onSubmit={(e) => {
                e.preventDefault();
                deployTemplateMutation.mutate();
              }}
            >
              <DialogHeader>
                <DialogTitle>Deploy {selectedTemplate.name}</DialogTitle>
                <DialogDescription>{selectedTemplate.description}</DialogDescription>
              </DialogHeader>
              <div className="space-y-4 py-4">
                <div className="space-y-2">
                  <Label htmlFor="template-service-name">Service Name</Label>
                  <Input
                    id="template-service-name"
                    value={templateServiceName}
                    onChange={(e) => setTemplateServiceName(e.target.value)}
                    placeholder="my-service"
                    pattern="[a-z0-9-]+"
                    required
                  />
                  <p className="text-xs text-muted-foreground">
                    Lowercase letters, numbers, and hyphens only
                  </p>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="template-port">
                    Port
                    <span className="text-destructive ml-1">*</span>
                  </Label>
                  <Input
                    id="template-port"
                    type="number"
                    value={templateEnvVars["PORT"] || "8080"}
                    onChange={(e) =>
                      setTemplateEnvVars((prev) => ({ ...prev, PORT: e.target.value }))
                    }
                    placeholder="8080"
                    required
                  />
                  <p className="text-xs text-muted-foreground">
                    Container port to expose (use unique ports to avoid conflicts)
                  </p>
                </div>

                {selectedTemplate.env_schema && selectedTemplate.env_schema.length > 0 && (
                  <div className="space-y-4 pt-2">
                    <Label className="text-base">Configuration</Label>
                    {selectedTemplate.env_schema.map((entry) => (
                      <div key={entry.name} className="space-y-1">
                        <Label htmlFor={`template-env-${entry.name}`} className="text-sm">
                          {entry.label}
                          {entry.required && <span className="text-destructive ml-1">*</span>}
                        </Label>
                        <div className="relative">
                          <Input
                            id={`template-env-${entry.name}`}
                            type={
                              entry.secret && !showTemplateSecrets[entry.name]
                                ? "password"
                                : "text"
                            }
                            value={templateEnvVars[entry.name] || ""}
                            onChange={(e) =>
                              setTemplateEnvVars((prev) => ({
                                ...prev,
                                [entry.name]: e.target.value,
                              }))
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
                              onClick={() =>
                                setShowTemplateSecrets((prev) => ({
                                  ...prev,
                                  [entry.name]: !prev[entry.name],
                                }))
                              }
                            >
                              {showTemplateSecrets[entry.name] ? (
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
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => {
                    setSelectedTemplate(null);
                    setTemplateServiceName("");
                    setTemplateEnvVars({});
                    setShowTemplateSecrets({});
                  }}
                >
                  Back
                </Button>
                <Button type="submit" disabled={deployTemplateMutation.isPending}>
                  <Rocket className="mr-2 h-4 w-4" />
                  {deployTemplateMutation.isPending ? "Deploying..." : "Deploy Service"}
                </Button>
              </DialogFooter>
            </form>
          )}
        </DialogContent>
      </Dialog>
    </>
  );
}
