import React, { useState, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Badge } from "@/components/ui/badge";
import {
  ExternalLink,
  Lock,
  Globe,
  AlertCircle,
  Loader2,
  Check,
  ChevronsUpDown,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { api } from "@/lib/api";
import type { GitProvider, GitRepository } from "@/types/api";

// GitLab SVG icon (brand color #FC6D26)
const GitLabIcon = ({ className, style }: { className?: string; style?: React.CSSProperties }) => (
  <svg className={className} style={style} viewBox="0 0 24 24" fill="currentColor">
    <path d="M22.65 14.39L12 22.13 1.35 14.39a.84.84 0 01-.3-.94l1.22-3.78 2.44-7.51A.42.42 0 014.82 2a.43.43 0 01.58 0 .42.42 0 01.11.18l2.44 7.49h8.1l2.44-7.51A.42.42 0 0118.6 2a.43.43 0 01.58 0 .42.42 0 01.11.18l2.44 7.51L23 13.45a.84.84 0 01-.35.94z" />
  </svg>
);

export interface SelectedGitLabRepo {
  repository: GitRepository;
  providerId: string;
  gitUrl: string;
  branch: string;
}

interface GitLabRepoPickerProps {
  onSelect: (selection: SelectedGitLabRepo | null) => void;
  selectedRepoFullName?: string;
}

export function GitLabRepoPicker({
  onSelect,
  selectedRepoFullName,
}: GitLabRepoPickerProps) {
  const [providerId, setProviderId] = useState<string>("");
  const [repoFullName, setRepoFullName] = useState<string>(
    selectedRepoFullName || ""
  );
  const [branch, setBranch] = useState<string>("main");
  const [repoSearchOpen, setRepoSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");

  // Fetch all git providers, filter to GitLab ones
  const {
    data: allProviders = [],
    isLoading: isLoadingProviders,
    error: providersError,
  } = useQuery<GitProvider[]>({
    queryKey: ["git-providers"],
    queryFn: () => api.getGitProviders(),
  });

  const gitlabProviders = useMemo(
    () => allProviders.filter((p) => p.provider === "gitlab"),
    [allProviders]
  );

  // Auto-select first provider when list loads
  const effectiveProviderId =
    providerId || (gitlabProviders.length > 0 ? gitlabProviders[0].id : "");

  // Fetch repositories for the selected provider
  const {
    data: repositories = [],
    isLoading: isLoadingRepos,
    error: reposError,
  } = useQuery<GitRepository[]>({
    queryKey: ["git-provider-repos", effectiveProviderId],
    queryFn: () => api.getGitProviderRepos(effectiveProviderId, 1, 100),
    enabled: !!effectiveProviderId,
  });

  // Filter repositories based on search query
  const filteredRepos = useMemo(() => {
    if (!searchQuery) return repositories;
    const query = searchQuery.toLowerCase();
    return repositories.filter(
      (repo) =>
        repo.name.toLowerCase().includes(query) ||
        repo.full_name.toLowerCase().includes(query) ||
        (repo.description?.toLowerCase().includes(query) ?? false)
    );
  }, [repositories, searchQuery]);

  const selectedRepo = repositories.find((r) => r.full_name === repoFullName);

  const handleRepoSelect = (fullName: string) => {
    setRepoFullName(fullName);
    setRepoSearchOpen(false);
    setSearchQuery("");
    const repo = repositories.find((r) => r.full_name === fullName);
    if (repo) {
      const defaultBranch = repo.default_branch || "main";
      setBranch(defaultBranch);
      onSelect({
        repository: repo,
        providerId: effectiveProviderId,
        gitUrl: repo.clone_url,
        branch: defaultBranch,
      });
    }
  };

  // Show loading state
  if (isLoadingProviders) {
    return (
      <div className="flex items-center gap-2 text-muted-foreground p-4 border rounded-lg">
        <Loader2 className="h-4 w-4 animate-spin" />
        <span>Loading GitLab connections...</span>
      </div>
    );
  }

  // Show error state
  if (providersError) {
    return (
      <div className="p-4 border border-destructive/50 rounded-lg bg-destructive/10">
        <div className="flex items-center gap-2 text-destructive">
          <AlertCircle className="h-4 w-4" />
          <span>Failed to load GitLab connections</span>
        </div>
      </div>
    );
  }

  // Show "Connect GitLab" if no providers
  if (gitlabProviders.length === 0) {
    return (
      <div className="p-4 border rounded-lg bg-muted/50 space-y-3">
        <div className="flex items-center gap-2 text-muted-foreground">
          <GitLabIcon className="h-5 w-5" style={{ color: "#FC6D26" }} />
          <span className="font-medium">No GitLab connections</span>
        </div>
        <p className="text-sm text-muted-foreground">
          Connect your GitLab account to select repositories automatically.
        </p>
        <Button
          onClick={() =>
            (window.location.href =
              "/git-providers?tab=gitlab&action=connect")
          }
          className="gap-2"
        >
          <GitLabIcon className="h-4 w-4" />
          Connect GitLab
        </Button>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Provider selector — only shown when there are multiple GitLab providers */}
      {gitlabProviders.length > 1 && (
        <div className="space-y-2">
          <Label>GitLab Account</Label>
          <div className="flex flex-wrap gap-2">
            {gitlabProviders.map((provider) => (
              <button
                key={provider.id}
                type="button"
                onClick={() => {
                  setProviderId(provider.id);
                  setRepoFullName("");
                  setBranch("main");
                  onSelect(null);
                }}
                className={cn(
                  "flex items-center gap-2 px-3 py-1.5 rounded-md border text-sm transition-colors",
                  (providerId || gitlabProviders[0].id) === provider.id
                    ? "border-primary bg-primary/5"
                    : "border-border hover:border-muted-foreground/50"
                )}
              >
                <GitLabIcon
                  className="h-4 w-4"
                  style={{ color: "#FC6D26" }}
                />
                <span>{provider.display_name || provider.username}</span>
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Repository Selector with Search */}
      <div className="space-y-2">
        <Label>Repository</Label>
        {isLoadingRepos ? (
          <div className="flex items-center gap-2 text-muted-foreground py-2">
            <Loader2 className="h-4 w-4 animate-spin" />
            <span>Loading repositories...</span>
          </div>
        ) : reposError ? (
          <div className="flex items-center gap-2 text-destructive py-2">
            <AlertCircle className="h-4 w-4" />
            <span>Failed to load repositories</span>
          </div>
        ) : repositories.length === 0 ? (
          <div className="text-sm text-muted-foreground py-2">
            No repositories found. Make sure your GitLab token has the
            necessary permissions.
          </div>
        ) : (
          <Popover open={repoSearchOpen} onOpenChange={setRepoSearchOpen}>
            <PopoverTrigger asChild>
              <Button
                variant="outline"
                role="combobox"
                aria-expanded={repoSearchOpen}
                className="w-full justify-between font-normal"
              >
                {selectedRepo ? (
                  <div className="flex items-center gap-2">
                    {selectedRepo.private ? (
                      <Lock className="h-3 w-3 text-muted-foreground" />
                    ) : (
                      <Globe className="h-3 w-3 text-muted-foreground" />
                    )}
                    <span>{selectedRepo.name}</span>
                    <span className="text-muted-foreground text-xs">
                      ({selectedRepo.default_branch})
                    </span>
                  </div>
                ) : (
                  <span className="text-muted-foreground">
                    Search repositories...
                  </span>
                )}
                <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
              </Button>
            </PopoverTrigger>
            <PopoverContent className="w-100 p-0" align="start">
              <Command shouldFilter={false}>
                <CommandInput
                  placeholder="Search repositories..."
                  value={searchQuery}
                  onValueChange={setSearchQuery}
                />
                <CommandList>
                  <CommandEmpty>No repositories found.</CommandEmpty>
                  <CommandGroup>
                    {filteredRepos.map((repo) => (
                      <CommandItem
                        key={repo.id}
                        value={repo.full_name}
                        onSelect={handleRepoSelect}
                        className="cursor-pointer"
                      >
                        <Check
                          className={cn(
                            "mr-2 h-4 w-4",
                            repoFullName === repo.full_name
                              ? "opacity-100"
                              : "opacity-0"
                          )}
                        />
                        <div className="flex items-center gap-2 flex-1 min-w-0">
                          {repo.private ? (
                            <Lock className="h-3 w-3 text-muted-foreground shrink-0" />
                          ) : (
                            <Globe className="h-3 w-3 text-muted-foreground shrink-0" />
                          )}
                          <div className="flex flex-col min-w-0">
                            <span className="font-medium truncate">
                              {repo.name}
                            </span>
                            {repo.description && (
                              <span className="text-xs text-muted-foreground truncate">
                                {repo.description}
                              </span>
                            )}
                          </div>
                          <Badge
                            variant="outline"
                            className="ml-auto text-xs shrink-0"
                          >
                            {repo.default_branch}
                          </Badge>
                        </div>
                      </CommandItem>
                    ))}
                  </CommandGroup>
                </CommandList>
              </Command>
            </PopoverContent>
          </Popover>
        )}
      </div>

      {/* Branch input */}
      {selectedRepo && (
        <div className="space-y-2">
          <Label htmlFor="gitlab-branch">Branch</Label>
          <input
            id="gitlab-branch"
            type="text"
            className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            value={branch}
            onChange={(e) => {
              setBranch(e.target.value);
              if (selectedRepo) {
                onSelect({
                  repository: selectedRepo,
                  providerId: effectiveProviderId,
                  gitUrl: selectedRepo.clone_url,
                  branch: e.target.value,
                });
              }
            }}
          />
        </div>
      )}

      {/* Selected repo info */}
      {selectedRepo && (
        <div className="p-3 border rounded-lg bg-muted/30 space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium">{selectedRepo.full_name}</span>
            <a
              href={selectedRepo.html_url}
              target="_blank"
              rel="noopener noreferrer"
              className="text-muted-foreground hover:text-foreground"
            >
              <ExternalLink className="h-4 w-4" />
            </a>
          </div>
          {selectedRepo.description && (
            <p className="text-xs text-muted-foreground">
              {selectedRepo.description}
            </p>
          )}
        </div>
      )}
    </div>
  );
}

export default GitLabRepoPicker;
