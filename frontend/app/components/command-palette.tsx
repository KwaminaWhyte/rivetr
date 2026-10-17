import { useEffect, useMemo } from "react";
import { useNavigate } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { Box, Database, FolderKanban, Layers, Plus, CornerDownRight } from "lucide-react";

import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
  Command,
} from "@/components/ui/command";
import { api } from "@/lib/api";
import { allNavPages } from "@/lib/nav";
import { useIsInstanceAdmin } from "@/lib/auth";
import { useTeamContext } from "@/lib/team-context";
import type { App, ManagedDatabase, Project, Service } from "@/types/api";

interface CommandPaletteProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/**
 * Global Cmd+K / Ctrl+K palette: jump to any page, app, project, database or
 * service, or run a quick action. Lists are only fetched while it is open and
 * share query keys with the rest of the dashboard.
 */
export function CommandPalette({ open, onOpenChange }: CommandPaletteProps) {
  const navigate = useNavigate();
  const { currentTeamId } = useTeamContext();
  const isAdmin = useIsInstanceAdmin();

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key.toLowerCase() === "k" && (e.metaKey || e.ctrlKey) && !e.altKey) {
        e.preventDefault();
        onOpenChange(!open);
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [open, onOpenChange]);

  const { data: projects = [] } = useQuery<Project[]>({
    queryKey: ["projects", currentTeamId],
    queryFn: () => api.getProjects(currentTeamId ?? undefined),
    enabled: open,
    staleTime: 30000,
  });

  const { data: apps = [] } = useQuery<App[]>({
    queryKey: ["apps", currentTeamId],
    queryFn: () => api.getApps(currentTeamId ? { teamId: currentTeamId } : undefined),
    enabled: open,
    staleTime: 30000,
  });

  const { data: databases = [] } = useQuery<ManagedDatabase[]>({
    queryKey: ["databases", currentTeamId],
    queryFn: () => api.getDatabases(currentTeamId ? { teamId: currentTeamId } : {}),
    enabled: open,
    staleTime: 30000,
  });

  const { data: services = [] } = useQuery<Service[]>({
    queryKey: ["services", currentTeamId],
    queryFn: () => api.getServices(currentTeamId ? { teamId: currentTeamId } : undefined),
    enabled: open,
    staleTime: 30000,
  });

  const pageGroups = useMemo(() => {
    const groups = new Map<string, { title: string; url: string }[]>();
    for (const page of allNavPages(isAdmin)) {
      const list = groups.get(page.group) ?? [];
      list.push(page);
      groups.set(page.group, list);
    }
    // Pages first, then the rest in definition order
    return [...groups.entries()].sort(([a], [b]) =>
      a === "Pages" ? -1 : b === "Pages" ? 1 : 0
    );
  }, [isAdmin]);

  const go = (url: string) => {
    onOpenChange(false);
    navigate(url);
  };

  return (
    <CommandDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Search"
      description="Jump to a page, app, project, database or service"
    >
      <Command>
        <CommandInput placeholder="Search pages, apps, projects..." />
        <CommandList>
          <CommandEmpty>No results found.</CommandEmpty>

          <CommandGroup heading="Actions">
            <CommandItem value="action new project create" onSelect={() => go("/projects?new=1")}>
              <Plus />
              <span>New project</span>
            </CommandItem>
          </CommandGroup>
          <CommandSeparator />

          {projects.length > 0 && (
            <CommandGroup heading="Projects">
              {projects.map((p) => (
                <CommandItem
                  key={p.id}
                  value={`project ${p.name} ${p.id}`}
                  onSelect={() => go(`/projects/${p.id}`)}
                >
                  <FolderKanban />
                  <span>{p.name}</span>
                </CommandItem>
              ))}
            </CommandGroup>
          )}

          {apps.length > 0 && (
            <CommandGroup heading="Apps">
              {apps.map((a) => (
                <CommandItem
                  key={a.id}
                  value={`app ${a.name} ${a.id}`}
                  onSelect={() => go(`/apps/${a.id}`)}
                >
                  <Box />
                  <span>{a.name}</span>
                </CommandItem>
              ))}
            </CommandGroup>
          )}

          {databases.length > 0 && (
            <CommandGroup heading="Databases">
              {databases.map((d) => (
                <CommandItem
                  key={d.id}
                  value={`database ${d.name} ${d.id}`}
                  onSelect={() => go(`/databases/${d.id}`)}
                >
                  <Database />
                  <span>{d.name}</span>
                </CommandItem>
              ))}
            </CommandGroup>
          )}

          {services.length > 0 && (
            <CommandGroup heading="Services">
              {services.map((s) => (
                <CommandItem
                  key={s.id}
                  value={`service ${s.name} ${s.id}`}
                  onSelect={() => go(`/services/${s.id}`)}
                >
                  <Layers />
                  <span>{s.name}</span>
                </CommandItem>
              ))}
            </CommandGroup>
          )}

          {pageGroups.map(([group, pages]) => (
            <CommandGroup key={group} heading={group}>
              {pages.map((page) => (
                <CommandItem
                  key={page.url}
                  value={`${group} ${page.title} ${page.url}`}
                  onSelect={() => go(page.url)}
                >
                  <CornerDownRight />
                  <span>{page.title}</span>
                </CommandItem>
              ))}
            </CommandGroup>
          ))}
        </CommandList>
      </Command>
    </CommandDialog>
  );
}
