import { useEffect, useState } from "react";
import { useOutletContext } from "react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { GitHubRepoPicker } from "@/components/github-repo-picker";
import { api } from "@/lib/api";
import type {
  Service,
  ServiceBuildImage,
  ServiceDeployConfig,
  ServiceDeployHook,
  ServiceSourceType,
} from "@/types/api";
import { Hammer, Plus, Trash2, BookOpen } from "lucide-react";
// HOOKS_IMPORT

export function meta() {
  return [
    { title: "Service Build - Rivetr" },
    { name: "description", content: "Build images from source before deploying" },
  ];
}

interface OutletContext {
  service: Service;
}

interface ImageDraft {
  name: string;
  image: string;
  dockerfile: string;
  context: string;
  target: string;
  /** KEY=VALUE per line */
  buildArgs: string;
}

function toDraft(img: ServiceBuildImage): ImageDraft {
  return {
    name: img.name ?? "",
    image: img.image,
    dockerfile: img.dockerfile,
    context: img.context,
    target: img.target ?? "",
    buildArgs: Object.entries(img.build_args ?? {})
      .map(([k, v]) => `${k}=${v}`)
      .join("\n"),
  };
}

function fromDraft(d: ImageDraft): ServiceBuildImage {
  const build_args: Record<string, string> = {};
  for (const line of d.buildArgs.split("\n")) {
    const t = line.trim();
    if (!t) continue;
    const eq = t.indexOf("=");
    if (eq <= 0) continue;
    build_args[t.slice(0, eq).trim()] = t.slice(eq + 1);
  }
  return {
    name: d.name.trim() || undefined,
    image: d.image.trim(),
    dockerfile: d.dockerfile.trim() || "Dockerfile",
    context: d.context.trim() || ".",
    target: d.target.trim() || undefined,
    build_args,
  };
}

const emptyImage: ImageDraft = {
  name: "",
  image: "",
  dockerfile: "Dockerfile",
  context: ".",
  target: "",
  buildArgs: "",
};

export default function ServiceBuildTab() {
  const { service } = useOutletContext<OutletContext>();
  const queryClient = useQueryClient();

  const { data: config, isLoading } = useQuery<ServiceDeployConfig>({
    queryKey: ["service-deploy-config", service.id],
    queryFn: () => api.getServiceDeployConfig(service.id),
  });

  const [sourceType, setSourceType] = useState<ServiceSourceType>("none");
  const [gitUrl, setGitUrl] = useState("");
  const [branch, setBranch] = useState("main");
  const [installationId, setInstallationId] = useState("");
  const [autoDeploy, setAutoDeploy] = useState(false);
  const [sourcePath, setSourcePath] = useState("");
  const [revisionFile, setRevisionFile] = useState("");
  const [images, setImages] = useState<ImageDraft[]>([]);
  const [pushImages, setPushImages] = useState(true);
  const [registryUser, setRegistryUser] = useState("");
  const [registryPassword, setRegistryPassword] = useState("");
  const [preHooks, setPreHooks] = useState<ServiceDeployHook[]>([]);
  const [postHooks, setPostHooks] = useState<ServiceDeployHook[]>([]);
  const [showPicker, setShowPicker] = useState(false);

  useEffect(() => {
    if (!config) return;
    setSourceType(config.source_type);
    setGitUrl(config.git_url ?? "");
    setBranch(config.branch || "main");
    setInstallationId(config.github_app_installation_id ?? "");
    setAutoDeploy(config.auto_deploy);
    setSourcePath(config.source_path ?? "");
    setRevisionFile(config.revision_file ?? "");
    setImages(config.images.map(toDraft));
    setPushImages(config.push_images);
    setRegistryUser(config.registry_username ?? "");
    setRegistryPassword("");
    setPreHooks(config.pre_deploy_hooks);
    setPostHooks(config.post_deploy_hooks);
  }, [config]);

  const saveMutation = useMutation({
    mutationFn: () =>
      api.updateServiceDeployConfig(service.id, {
        source_type: sourceType,
        git_url: gitUrl,
        branch,
        github_app_installation_id: installationId,
        auto_deploy: sourceType === "git" ? autoDeploy : false,
        source_path: sourcePath,
        revision_file: revisionFile,
        images: sourceType === "none" ? [] : images.map(fromDraft),
        push_images: pushImages,
        registry_username: registryUser,
        ...(registryPassword ? { registry_password: registryPassword } : {}),
        pre_deploy_hooks: preHooks,
        post_deploy_hooks: postHooks,
      }),
    onSuccess: (data) => {
      queryClient.setQueryData(["service-deploy-config", service.id], data);
      toast.success("Build settings saved");
    },
    onError: (e) => toast.error(e instanceof Error ? e.message : "Failed to save"),
  });

  const clearPasswordMutation = useMutation({
    mutationFn: () => api.updateServiceDeployConfig(service.id, { registry_password: "" }),
    onSuccess: (data) => {
      queryClient.setQueryData(["service-deploy-config", service.id], data);
      toast.success("Registry password removed");
    },
  });

  if (isLoading || !config) {
    return <div className="h-48 animate-pulse rounded-lg bg-muted" />;
  }

  const updateImage = (i: number, patch: Partial<ImageDraft>) =>
    setImages((prev) => prev.map((img, idx) => (idx === i ? { ...img, ...patch } : img)));

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Hammer className="h-5 w-5" />
            Build from source
          </CardTitle>
          <CardDescription>
            Without a source, Deploy pulls the images named in the compose file. With one,
            Deploy fetches the source, builds each image below tagged with the revision and{" "}
            <code>latest</code>, pushes both tags, runs the pre-deploy hooks and then{" "}
            <code>docker compose up -d</code>.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-2 sm:max-w-xs">
            <Label>Source</Label>
            <Select value={sourceType} onValueChange={(v) => setSourceType(v as ServiceSourceType)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="none">None (pull images only)</SelectItem>
                <SelectItem value="git">Git repository</SelectItem>
                <SelectItem value="path">Path on the server</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {sourceType === "git" && (
            <div className="space-y-4">
              <div className="grid gap-4 sm:grid-cols-2">
                <div className="grid gap-2">
                  <Label htmlFor="git-url">Repository URL</Label>
                  <Input
                    id="git-url"
                    value={gitUrl}
                    onChange={(e) => setGitUrl(e.target.value)}
                    placeholder="https://github.com/acme/shop.git"
                  />
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="branch">Branch</Label>
                  <Input id="branch" value={branch} onChange={(e) => setBranch(e.target.value)} />
                </div>
              </div>
              <p className="text-xs text-muted-foreground">
                Private repositories clone with a GitHub App installation (pick the repository
                below) or the global SSH key for SSH URLs, the same way apps do.
                {installationId && " A GitHub App installation is linked."}
              </p>
              {showPicker ? (
                <GitHubRepoPicker
                  selectedInstallationId={installationId || undefined}
                  onSelect={(sel) => {
                    if (!sel) return;
                    setInstallationId(sel.installationId);
                    setGitUrl(sel.gitUrl);
                    if (sel.branch) setBranch(sel.branch);
                  }}
                />
              ) : (
                <Button variant="outline" size="sm" onClick={() => setShowPicker(true)}>
                  Pick a repository from a GitHub App
                </Button>
              )}
              <div className="flex items-center gap-3">
                <Switch id="auto-deploy" checked={autoDeploy} onCheckedChange={setAutoDeploy} />
                <Label htmlFor="auto-deploy">Deploy on push to {branch || "the branch"}</Label>
              </div>
              {autoDeploy && (
                <p className="text-xs text-muted-foreground">
                  Uses the same push webhooks as apps: a connected GitHub App, or a repository
                  webhook pointing at <code>/webhooks/github</code> (also GitLab and Gitea) on
                  this Rivetr instance.
                </p>
              )}
            </div>
          )}

          {sourceType === "path" && (
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="grid gap-2">
                <Label htmlFor="source-path">Source directory</Label>
                <Input
                  id="source-path"
                  value={sourcePath}
                  onChange={(e) => setSourcePath(e.target.value)}
                  placeholder="/opt/megasign/src"
                />
              </div>
              <div className="grid gap-2">
                <Label htmlFor="revision-file">Revision file (optional)</Label>
                <Input
                  id="revision-file"
                  value={revisionFile}
                  onChange={(e) => setRevisionFile(e.target.value)}
                  placeholder="/opt/megasign/REVISION"
                />
                <p className="text-xs text-muted-foreground">
                  Its content becomes the image tag. Without it, a git checkout uses the commit,
                  anything else a timestamp.
                </p>
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {sourceType !== "none" && (
        <Card>
          <CardHeader>
            <CardTitle>Images</CardTitle>
            <CardDescription>
              Built in order, so a later image can use an earlier one as its base. Paths are
              relative to the source directory unless absolute. Build args may use{" "}
              <code>{"${TAG}"}</code>, <code>{"${COMMIT_SHA}"}</code> and{" "}
              <code>{"${SHORT_SHA}"}</code>.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {images.map((img, i) => (
              <div key={i} className="space-y-3 rounded-lg border p-4">
                <div className="flex items-center justify-between gap-2">
                  <span className="text-sm font-medium">Image {i + 1}</span>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setImages((prev) => prev.filter((_, idx) => idx !== i))}
                    aria-label="Remove image"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
                <div className="grid gap-3 sm:grid-cols-2">
                  <div className="grid gap-1.5">
                    <Label>Target repository</Label>
                    <Input
                      value={img.image}
                      onChange={(e) => updateImage(i, { image: e.target.value })}
                      placeholder="localhost:5000/megasign-app"
                    />
                  </div>
                  <div className="grid gap-1.5">
                    <Label>Label (optional)</Label>
                    <Input
                      value={img.name}
                      onChange={(e) => updateImage(i, { name: e.target.value })}
                      placeholder="app"
                    />
                  </div>
                  <div className="grid gap-1.5">
                    <Label>Dockerfile</Label>
                    <Input
                      value={img.dockerfile}
                      onChange={(e) => updateImage(i, { dockerfile: e.target.value })}
                      placeholder="infra/Dockerfile"
                    />
                  </div>
                  <div className="grid gap-1.5">
                    <Label>Context</Label>
                    <Input
                      value={img.context}
                      onChange={(e) => updateImage(i, { context: e.target.value })}
                      placeholder="."
                    />
                  </div>
                  <div className="grid gap-1.5">
                    <Label>Target stage (optional)</Label>
                    <Input
                      value={img.target}
                      onChange={(e) => updateImage(i, { target: e.target.value })}
                    />
                  </div>
                  <div className="grid gap-1.5">
                    <Label>Build args (KEY=VALUE per line)</Label>
                    <Textarea
                      rows={2}
                      className="font-mono text-xs"
                      value={img.buildArgs}
                      onChange={(e) => updateImage(i, { buildArgs: e.target.value })}
                      placeholder={"BASE=localhost:5000/megasign-app:${TAG}"}
                    />
                  </div>
                </div>
              </div>
            ))}
            <Button
              variant="outline"
              size="sm"
              className="gap-2"
              onClick={() => setImages((prev) => [...prev, { ...emptyImage }])}
            >
              <Plus className="h-4 w-4" />
              Add image
            </Button>
          </CardContent>
        </Card>
      )}

      {sourceType !== "none" && (
        <Card>
          <CardHeader>
            <CardTitle>Registry</CardTitle>
            <CardDescription>
              Images are pushed to the registry in their name, e.g. a local{" "}
              <code>registry:2</code> at <code>localhost:5000</code> (no login needed).
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center gap-3">
              <Switch id="push" checked={pushImages} onCheckedChange={setPushImages} />
              <Label htmlFor="push">Push images after building</Label>
            </div>
            {pushImages && (
              <div className="grid gap-4 sm:grid-cols-2">
                <div className="grid gap-2">
                  <Label htmlFor="reg-user">Username (optional)</Label>
                  <Input
                    id="reg-user"
                    value={registryUser}
                    onChange={(e) => setRegistryUser(e.target.value)}
                    autoComplete="off"
                  />
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="reg-pass">Password or token</Label>
                  <Input
                    id="reg-pass"
                    type="password"
                    value={registryPassword}
                    onChange={(e) => setRegistryPassword(e.target.value)}
                    placeholder={config.has_registry_password ? "Stored (leave empty to keep)" : ""}
                    autoComplete="new-password"
                  />
                  {config.has_registry_password && (
                    <Button
                      variant="link"
                      size="sm"
                      className="h-auto justify-start p-0"
                      onClick={() => clearPasswordMutation.mutate()}
                    >
                      Remove stored password
                    </Button>
                  )}
                </div>
              </div>
            )}
          </CardContent>
        </Card>
      )}

      {/* HOOKS_SLOT */}

      <div className="flex justify-end">
        <Button onClick={() => saveMutation.mutate()} disabled={saveMutation.isPending}>
          {saveMutation.isPending ? "Saving..." : "Save build settings"}
        </Button>
      </div>

      <MigrationHelp service={service} />
    </div>
  );
}

function MigrationHelp({ service }: { service: Service }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <BookOpen className="h-5 w-5" />
          Moving a hand-built setup here
        </CardTitle>
        <CardDescription>
          For a service whose images are built by a script on the server and pushed to a local
          registry.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-2 text-sm">
        <ol className="list-decimal space-y-1 pl-5">
          <li>
            Keep the compose file as it is: it already names the images, e.g.{" "}
            <code>localhost:5000/app:latest</code>.
          </li>
          <li>
            Pick the source: the git repository and branch, or the directory the script builds
            from (with its revision file, if any).
          </li>
          <li>
            Add one image per <code>docker build</code> in the script, in the same order, with
            the same Dockerfile, context and build args. Replace the script&apos;s revision
            variable with <code>{"${TAG}"}</code>.
          </li>
          <li>
            Add each step the script runs between build and restart (a database migration) as a
            pre-deploy hook in the compose service that has the right env.
          </li>
          <li>Save, then Deploy and follow the log in the Deployments tab.</li>
        </ol>
        <p className="text-muted-foreground">
          Services without a build source keep deploying by pulling images. Service:{" "}
          <code>{service.name}</code>.
        </p>
      </CardContent>
    </Card>
  );
}
