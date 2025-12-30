import { useState, useMemo, useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
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
import { Github, ExternalLink, Lock, Globe, AlertCircle, Loader2, Check, ChevronsUpDown, Search, GitBranch, Shield } from "lucide-react";
import { cn } from "@/lib/utils";
import { api } from "@/lib/api";
import type { GitHubAppInstallation, GitHubAppRepository, GitHubBranch } from "@/types/api";

export interface SelectedRepo {
  repository: GitHubAppRepository;
  installationId: string;
  gitUrl: string;
  branch: string;
}

interface GitHubRepoPickerProps {
  onSelect: (selection: SelectedRepo | null) => void;
  selectedInstallationId?: string;
  selectedRepoFullName?: string;
}

export function GitHubRepoPicker({
  onSelect,
  selectedInstallationId,
  selectedRepoFullName,
}: GitHubRepoPickerProps) {
  const [installationId, setInstallationId] = useState<string>(selectedInstallationId || "");
  const [repoFullName, setRepoFullName] = useState<string>(selectedRepoFullName || "");
  const [selectedBranch, setSelectedBranch] = useState<string>("");
  const [repoSearchOpen, setRepoSearchOpen] = useState(false);
  const [branchSearchOpen, setBranchSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [branchSearchQuery, setBranchSearchQuery] = useState("");

  // Fetch all installations
  const {
    data: installations = [],
    isLoading: isLoadingInstallations,
    error: installationsError,
  } = useQuery<GitHubAppInstallation[]>({
    queryKey: ["github-app-installations-all"],
    queryFn: () => api.getAllGitHubAppInstallations(),
  });

  // Fetch repositories for selected installation
  const {
    data: repositories = [],
    isLoading: isLoadingRepos,
    error: reposError,
  } = useQuery<GitHubAppRepository[]>({
    queryKey: ["github-app-repos", installationId],
    queryFn: () => api.getGitHubAppRepositories(installationId, 1, 100),
    enabled: !!installationId,
  });

  // Get owner/repo from full_name for branches query
  const [repoOwner, repoName] = repoFullName ? repoFullName.split("/") : ["", ""];

  // Fetch branches for selected repository
  const {
    data: branches = [],
    isLoading: isLoadingBranches,
    error: branchesError,
  } = useQuery<GitHubBranch[]>({
    queryKey: ["github-app-branches", installationId, repoOwner, repoName],
    queryFn: () => api.getGitHubAppRepoBranches(installationId, repoOwner, repoName),
    enabled: !!installationId && !!repoOwner && !!repoName,
  });

  // Filter branches based on search query
  const filteredBranches = useMemo(() => {
    if (!branchSearchQuery) return branches;
    const query = branchSearchQuery.toLowerCase();
    return branches.filter((branch) => branch.name.toLowerCase().includes(query));
  }, [branches, branchSearchQuery]);

  // Fetch GitHub Apps for "Connect" button
  const { data: githubApps = [] } = useQuery({
    queryKey: ["github-apps"],
    queryFn: () => api.getGitHubApps(),
    enabled: installations.length === 0 && !isLoadingInstallations,
  });

  // Filter repositories based on search query
  const filteredRepos = useMemo(() => {
    if (!searchQuery) return repositories;
    const query = searchQuery.toLowerCase();
    return repositories.filter(
      (repo) =>
        repo.name.toLowerCase().includes(query) ||
        repo.full_name.toLowerCase().includes(query) ||
        repo.description?.toLowerCase().includes(query)
    );
  }, [repositories, searchQuery]);

  const handleInstallationChange = (value: string) => {
    setInstallationId(value);
    setRepoFullName("");
    setSelectedBranch("");
    setSearchQuery("");
    setBranchSearchQuery("");
    onSelect(null);
  };

  const handleRepoSelect = (fullName: string) => {
    setRepoFullName(fullName);
    setRepoSearchOpen(false);
    setBranchSearchQuery("");
    const repo = repositories.find((r) => r.full_name === fullName);
    if (repo) {
      // Set default branch initially, will be updated when branches load
      setSelectedBranch(repo.default_branch);
      onSelect({
        repository: repo,
        installationId,
        gitUrl: repo.clone_url,
        branch: repo.default_branch,
      });
    }
  };

  const handleBranchSelect = (branchName: string) => {
    setSelectedBranch(branchName);
    setBranchSearchOpen(false);
    const repo = repositories.find((r) => r.full_name === repoFullName);
    if (repo) {
      onSelect({
        repository: repo,
        installationId,
        gitUrl: repo.clone_url,
        branch: branchName,
      });
    }
  };

  // Set default branch when branches load
  useEffect(() => {
    if (branches.length > 0 && repoFullName && !selectedBranch) {
      const repo = repositories.find((r) => r.full_name === repoFullName);
      if (repo) {
        setSelectedBranch(repo.default_branch);
      }
    }
  }, [branches, repoFullName, selectedBranch, repositories]);

  const handleConnectGitHub = async () => {
    if (githubApps.length > 0) {
      // Use the first available GitHub App
      try {
        const { install_url } = await api.getGitHubAppInstallUrl(githubApps[0].id);
        window.location.href = install_url;
      } catch (error) {
        console.error("Failed to get install URL:", error);
      }
    } else {
      // Start the GitHub App creation flow
      window.location.href = "/settings/github-apps?action=create";
    }
  };

  const selectedRepo = repositories.find((r) => r.full_name === repoFullName);

  // Show loading state
  if (isLoadingInstallations) {
    return (
      <div className="flex items-center gap-2 text-muted-foreground p-4 border rounded-lg">
        <Loader2 className="h-4 w-4 animate-spin" />
        <span>Loading GitHub connections...</span>
      </div>
    );
  }

  // Show error state
  if (installationsError) {
    return (
      <div className="p-4 border border-destructive/50 rounded-lg bg-destructive/10">
        <div className="flex items-center gap-2 text-destructive">
          <AlertCircle className="h-4 w-4" />
          <span>Failed to load GitHub connections</span>
        </div>
      </div>
    );
  }

  // Show "Connect GitHub" if no installations
  if (installations.length === 0) {
    return (
      <div className="p-4 border rounded-lg bg-muted/50 space-y-3">
        <div className="flex items-center gap-2 text-muted-foreground">
          <Github className="h-5 w-5" />
          <span className="font-medium">No GitHub connections</span>
        </div>
        <p className="text-sm text-muted-foreground">
          Connect your GitHub account to select repositories automatically.
        </p>
        <Button onClick={handleConnectGitHub} className="gap-2">
          <Github className="h-4 w-4" />
          Connect GitHub
        </Button>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Installation Selector */}
      <div className="space-y-2">
        <Label>GitHub Account</Label>
        <Select value={installationId} onValueChange={handleInstallationChange}>
          <SelectTrigger className="w-full">
            <SelectValue placeholder="Select GitHub account" />
          </SelectTrigger>
          <SelectContent>
            {installations.map((installation) => (
              <SelectItem key={installation.id} value={installation.id}>
                <div className="flex items-center gap-2">
                  <Github className="h-4 w-4" />
                  <span>{installation.account_login}</span>
                  <Badge variant="outline" className="text-xs">
                    {installation.account_type === "organization" ? "Org" : "User"}
                  </Badge>
                  {installation.repository_selection === "all" ? (
                    <Badge variant="secondary" className="text-xs">All repos</Badge>
                  ) : (
                    <Badge variant="outline" className="text-xs">Selected repos</Badge>
                  )}
                </div>
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {/* Repository Selector with Search */}
      {installationId && (
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
              No repositories found. Make sure the GitHub App has access to your repositories.
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
                    <span className="text-muted-foreground">Search repositories...</span>
                  )}
                  <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
                </Button>
              </PopoverTrigger>
              <PopoverContent className="w-[400px] p-0" align="start">
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
                              repoFullName === repo.full_name ? "opacity-100" : "opacity-0"
                            )}
                          />
                          <div className="flex items-center gap-2 flex-1 min-w-0">
                            {repo.private ? (
                              <Lock className="h-3 w-3 text-muted-foreground shrink-0" />
                            ) : (
                              <Globe className="h-3 w-3 text-muted-foreground shrink-0" />
                            )}
                            <div className="flex flex-col min-w-0">
                              <span className="font-medium truncate">{repo.name}</span>
                              {repo.description && (
                                <span className="text-xs text-muted-foreground truncate">
                                  {repo.description}
                                </span>
                              )}
                            </div>
                            <Badge variant="outline" className="ml-auto text-xs shrink-0">
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
      )}

      {/* Branch Selector */}
      {selectedRepo && (
        <div className="space-y-2">
          <Label>Branch</Label>
          {isLoadingBranches ? (
            <div className="flex items-center gap-2 text-muted-foreground py-2">
              <Loader2 className="h-4 w-4 animate-spin" />
              <span>Loading branches...</span>
            </div>
          ) : branchesError ? (
            <div className="flex items-center gap-2 text-destructive py-2">
              <AlertCircle className="h-4 w-4" />
              <span>Failed to load branches</span>
            </div>
          ) : branches.length === 0 ? (
            <div className="text-sm text-muted-foreground py-2">
              No branches found.
            </div>
          ) : (
            <Popover open={branchSearchOpen} onOpenChange={setBranchSearchOpen}>
              <PopoverTrigger asChild>
                <Button
                  variant="outline"
                  role="combobox"
                  aria-expanded={branchSearchOpen}
                  className="w-full justify-between font-normal"
                >
                  {selectedBranch ? (
                    <div className="flex items-center gap-2">
                      <GitBranch className="h-3 w-3 text-muted-foreground" />
                      <span>{selectedBranch}</span>
                      {branches.find(b => b.name === selectedBranch)?.is_protected && (
                        <Shield className="h-3 w-3 text-amber-500" title="Protected branch" />
                      )}
                      {selectedBranch === selectedRepo.default_branch && (
                        <Badge variant="secondary" className="text-xs">default</Badge>
                      )}
                    </div>
                  ) : (
                    <span className="text-muted-foreground">Select branch...</span>
                  )}
                  <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
                </Button>
              </PopoverTrigger>
              <PopoverContent className="w-[300px] p-0" align="start">
                <Command shouldFilter={false}>
                  <CommandInput
                    placeholder="Search branches..."
                    value={branchSearchQuery}
                    onValueChange={setBranchSearchQuery}
                  />
                  <CommandList>
                    <CommandEmpty>No branches found.</CommandEmpty>
                    <CommandGroup>
                      {filteredBranches.map((branch) => (
                        <CommandItem
                          key={branch.name}
                          value={branch.name}
                          onSelect={handleBranchSelect}
                          className="cursor-pointer"
                        >
                          <Check
                            className={cn(
                              "mr-2 h-4 w-4",
                              selectedBranch === branch.name ? "opacity-100" : "opacity-0"
                            )}
                          />
                          <div className="flex items-center gap-2 flex-1">
                            <GitBranch className="h-3 w-3 text-muted-foreground" />
                            <span>{branch.name}</span>
                            {branch.is_protected && (
                              <Shield className="h-3 w-3 text-amber-500" title="Protected" />
                            )}
                            {branch.name === selectedRepo.default_branch && (
                              <Badge variant="secondary" className="text-xs ml-auto">default</Badge>
                            )}
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
            <p className="text-xs text-muted-foreground">{selectedRepo.description}</p>
          )}
        </div>
      )}
    </div>
  );
}

export default GitHubRepoPicker;
