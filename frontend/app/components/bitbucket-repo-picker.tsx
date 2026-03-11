import { useState, useMemo } from "react";
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

const BitbucketIcon = ({ className }: { className?: string }) => (
  <svg className={className} viewBox="0 0 24 24" fill="currentColor">
    <path d="M.778 1.213a.768.768 0 00-.768.892l3.263 19.81c.084.5.515.868 1.022.873H19.95a.772.772 0 00.77-.646l3.27-20.03a.768.768 0 00-.768-.889zM14.52 15.53H9.522L8.17 8.466h7.561z" />
  </svg>
);

export interface SelectedBitbucketRepo {
  repository: GitRepository;
  providerId: string;
  gitUrl: string;
  branch: string;
}

interface BitbucketRepoPickerProps {
  onSelect: (selection: SelectedBitbucketRepo | null) => void;
  selectedRepoFullName?: string;
}

export function BitbucketRepoPicker({
  onSelect,
  selectedRepoFullName,
}: BitbucketRepoPickerProps) {
  const [providerId, setProviderId] = useState<string>("");
  const [repoFullName, setRepoFullName] = useState<string>(
    selectedRepoFullName || ""
  );
  const [branch, setBranch] = useState<string>("main");
  const [repoSearchOpen, setRepoSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");

  const {
    data: allProviders = [],
    isLoading: isLoadingProviders,
    error: providersError,
  } = useQuery<GitProvider[]>({
    queryKey: ["git-providers"],
    queryFn: () => api.getGitProviders(),
  });

  const bitbucketProviders = useMemo(
    () => allProviders.filter((p) => p.provider === "bitbucket"),
    [allProviders]
  );

  const effectiveProviderId =
    providerId || (bitbucketProviders.length > 0 ? bitbucketProviders[0].id : "");

  const {
    data: repositories = [],
    isLoading: isLoadingRepos,
    error: reposError,
  } = useQuery<GitRepository[]>({
    queryKey: ["git-provider-repos", effectiveProviderId],
    queryFn: () => api.getGitProviderRepos(effectiveProviderId, 1, 100),
    enabled: !!effectiveProviderId,
  });

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

  if (isLoadingProviders) {
    return (
      <div className="flex items-center gap-2 text-muted-foreground p-4 border rounded-lg">
        <Loader2 className="h-4 w-4 animate-spin" />
        <span>Loading Bitbucket connections...</span>
      </div>
    );
  }

  if (providersError) {
    return (
      <div className="p-4 border border-destructive/50 rounded-lg bg-destructive/10">
        <div className="flex items-center gap-2 text-destructive">
          <AlertCircle className="h-4 w-4" />
          <span>Failed to load Bitbucket connections</span>
        </div>
      </div>
    );
  }

  if (bitbucketProviders.length === 0) {
    return (
      <div className="p-4 border rounded-lg bg-muted/50 space-y-3">
        <div className="flex items-center gap-2 text-muted-foreground">
          <BitbucketIcon className="h-5 w-5" />
          <span className="font-medium">No Bitbucket connections</span>
        </div>
        <p className="text-sm text-muted-foreground">
          Connect your Bitbucket account to select repositories automatically.
        </p>
        <Button
          onClick={() =>
            (window.location.href =
              "/git-providers?tab=token&action=connect&provider=bitbucket")
          }
          className="gap-2"
        >
          <BitbucketIcon className="h-4 w-4" />
          Connect Bitbucket
        </Button>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {bitbucketProviders.length > 1 && (
        <div className="space-y-2">
          <Label>Bitbucket Account</Label>
          <div className="flex flex-wrap gap-2">
            {bitbucketProviders.map((provider) => (
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
                  (providerId || bitbucketProviders[0].id) === provider.id
                    ? "border-primary bg-primary/5"
                    : "border-border hover:border-muted-foreground/50"
                )}
              >
                <BitbucketIcon className="h-4 w-4 text-[#0052CC]" />
                <span>{provider.display_name || provider.username}</span>
              </button>
            ))}
          </div>
        </div>
      )}

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
            No repositories found. Make sure your Bitbucket token has the
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

      {selectedRepo && (
        <div className="space-y-2">
          <Label htmlFor="bitbucket-branch">Branch</Label>
          <input
            id="bitbucket-branch"
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

export default BitbucketRepoPicker;
