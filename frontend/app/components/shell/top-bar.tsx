import { Fragment, type ReactNode } from "react";
import { Link, useLocation } from "react-router";
import { Box, Database, Layers, Plus, Settings2, Users } from "lucide-react";

import { DropdownMenuItem } from "@/components/ui/dropdown-menu";
import { SidebarTrigger } from "@/components/ui/sidebar";
import { CrumbSeparator, CrumbSwitcher, type CrumbOption } from "@/components/shell/crumb-switcher";
import { TopBarTarget } from "@/components/shell/top-bar-slot";
import { useBreadcrumb, type BreadcrumbItem } from "@/lib/breadcrumb-context";
import { getDefaultBreadcrumbs } from "@/lib/default-breadcrumbs";
import {
  activeResourceSection,
  matchResource,
  type ResourceMatch,
  type ResourceNavOptions,
} from "@/lib/resource-nav";
import {
  useProjectDetail,
  useProjectEnvironments,
  useProjectList,
  useResourceInfo,
  useServers,
} from "@/lib/resource-queries";
import { useTeamContext } from "@/lib/team-context";
import type { ProjectWithApps } from "@/types/api";

function TeamCrumb() {
  const { teams, currentTeamId, currentTeam, setCurrentTeamId } = useTeamContext();
  return (
    <CrumbSwitcher
      label={currentTeam?.name ?? "Personal"}
      icon={<Users className="size-3.5 text-muted-foreground" aria-hidden="true" />}
      heading="Teams"
      footer={
        <>
          {teams.map((team) => (
            <DropdownMenuItem key={team.id} onClick={() => setCurrentTeamId(team.id)}>
              <span className="flex size-5 items-center justify-center rounded border bg-muted text-[10px] font-semibold">
                {team.name.slice(0, 2).toUpperCase()}
              </span>
              <span className="min-w-0 flex-1 truncate">{team.name}</span>
              {team.id === currentTeamId && <span className="text-xs text-primary">Active</span>}
            </DropdownMenuItem>
          ))}
          <DropdownMenuItem asChild>
            <Link to="/teams?create=true">
              <Plus className="size-4" />
              Create team
            </Link>
          </DropdownMenuItem>
          <DropdownMenuItem asChild>
            <Link to="/teams">
              <Settings2 className="size-4" />
              Manage teams
            </Link>
          </DropdownMenuItem>
        </>
      }
    />
  );
}

function siblingResources(project?: ProjectWithApps): CrumbOption[] {
  if (!project) return [];
  return [
    ...project.apps.map((a) => ({
      id: a.id,
      label: a.name,
      href: `/apps/${a.id}`,
      hint: "App",
      icon: <Box className="size-3.5 text-muted-foreground" />,
    })),
    ...(project.databases ?? []).map((d) => ({
      id: d.id,
      label: d.name,
      href: `/databases/${d.id}`,
      hint: d.db_type,
      icon: <Database className="size-3.5 text-muted-foreground" />,
    })),
    ...(project.services ?? []).map((s) => ({
      id: s.id,
      label: s.name,
      href: `/services/${s.id}`,
      hint: "Service",
      icon: <Layers className="size-3.5 text-muted-foreground" />,
    })),
  ];
}

function ProjectCrumb({ projectId, current }: { projectId: string; current?: boolean }) {
  const { currentTeamId } = useTeamContext();
  const { data: project } = useProjectDetail(projectId);
  const { data: projects = [] } = useProjectList(currentTeamId);
  return (
    <CrumbSwitcher
      label={project?.name ?? "Project"}
      href={`/projects/${projectId}`}
      current={current}
      heading="Projects"
      activeId={projectId}
      options={projects.map((p) => ({ id: p.id, label: p.name, href: `/projects/${p.id}` }))}
      footer={
        <DropdownMenuItem asChild>
          <Link to="/projects?new=1">
            <Plus className="size-4" />
            New project
          </Link>
        </DropdownMenuItem>
      }
    />
  );
}

/** Crumbs after the resource itself: the section, then any detail page. */
function resourceSubCrumbs(
  match: ResourceMatch,
  pathname: string,
  options: ResourceNavOptions
): BreadcrumbItem[] {
  const path = pathname.replace(/\/+$/, "");
  if (path === match.base) return [];
  const section = activeResourceSection(match, pathname, options);
  if (!section || section.id === "general") return [{ label: "Page not found" }];
  if (path === section.url) return [{ label: section.title }];
  const detail = section.id === "deployments" ? "Deployment" : "Details";
  return [{ label: section.title, href: section.url }, { label: detail }];
}

function ResourceCrumbs({ match }: { match: ResourceMatch }) {
  const { pathname } = useLocation();
  const info = useResourceInfo(match);
  const { data: project } = useProjectDetail(info.projectId);
  const { data: environments = [] } = useProjectEnvironments(
    info.environmentId ? info.projectId : null
  );
  const { data: servers = [] } = useServers(match.kind === "server");
  const subCrumbs = resourceSubCrumbs(match, pathname, { dbType: info.variant });
  const sub = subCrumbs.length > 0;
  const env = environments.find((e) => e.id === info.environmentId);

  const crumbs: ReactNode[] = [];
  if (match.kind === "server") {
    crumbs.push(<CrumbSwitcher key="servers" label="Servers" href="/servers" />);
    crumbs.push(
      <CrumbSwitcher
        key="server"
        label={info.name ?? "Server"}
        href={match.base}
        current={!sub}
        heading="Servers"
        activeId={match.id}
        options={servers.map((s) => ({
          id: s.id,
          label: s.name,
          href: `/servers/${s.id}`,
          hint: s.host,
        }))}
      />
    );
  } else {
    if (info.projectId) {
      crumbs.push(<ProjectCrumb key="project" projectId={info.projectId} />);
    } else {
      crumbs.push(<CrumbSwitcher key="projects" label="Projects" href="/projects" />);
    }
    if (env && info.projectId) {
      crumbs.push(
        <CrumbSwitcher
          key="env"
          label={env.name}
          href={`/projects/${info.projectId}?env=${env.id}`}
          heading="Environments"
          activeId={env.id}
          options={environments.map((e) => ({
            id: e.id,
            label: e.name,
            href: `/projects/${info.projectId}?env=${e.id}`,
            hint: e.is_default ? "default" : undefined,
          }))}
        />
      );
    }
    crumbs.push(
      <CrumbSwitcher
        key="resource"
        label={info.name ?? "…"}
        href={match.base}
        current={!sub}
        heading="Resources in project"
        activeId={match.id}
        options={siblingResources(project)}
      />
    );
  }
  subCrumbs.forEach((c, i) => {
    crumbs.push(
      <CrumbSwitcher
        key={`sub-${i}`}
        label={c.label}
        href={c.href}
        current={i === subCrumbs.length - 1}
      />
    );
  });

  return (
    <>
      {crumbs.map((c, i) => (
        <Fragment key={i}>
          <CrumbSeparator />
          {c}
        </Fragment>
      ))}
    </>
  );
}

function PageCrumbs({ items }: { items: BreadcrumbItem[] }) {
  const { pathname } = useLocation();
  const projectMatch = pathname.match(/^\/projects\/([^/]+)/);
  const projectId = projectMatch && projectMatch[1] !== "new" ? projectMatch[1] : null;

  // Project pages: replace the generic "Projects / <name>" pair with a switcher.
  if (projectId) {
    const isProjectRoot = pathname.replace(/\/+$/, "") === `/projects/${projectId}`;
    const rest = isProjectRoot
      ? []
      : items
          .slice(1)
          .filter((item) => item.href !== `/projects/${projectId}` && item.label !== "Project");
    return (
      <>
        <CrumbSeparator />
        <ProjectCrumb projectId={projectId} current={isProjectRoot} />
        {!isProjectRoot &&
          rest.map((item, i) => (
            <Fragment key={i}>
              <CrumbSeparator />
              <CrumbSwitcher label={item.label} href={item.href} current={i === rest.length - 1} />
            </Fragment>
          ))}
      </>
    );
  }

  return (
    <>
      {items.map((item, i) => (
        <Fragment key={i}>
          <CrumbSeparator />
          <CrumbSwitcher
            label={item.label}
            href={item.href}
            current={i === items.length - 1}
            className={i < items.length - 1 ? "hidden sm:inline-flex" : undefined}
          />
        </Fragment>
      ))}
    </>
  );
}

/**
 * Shell top bar: team / project / environment / resource / section
 * breadcrumbs (switchers where the crumb has siblings), then the page's status pill, and the page's primary
 * actions on the right (both filled through TopBarPortal).
 */
export function TopBar({ crumbs: override }: { crumbs?: BreadcrumbItem[] } = {}) {
  const { pathname } = useLocation();
  const { items } = useBreadcrumb();
  const match = override ? null : matchResource(pathname);
  const crumbs = override ?? (items.length > 0 ? items : getDefaultBreadcrumbs(pathname));

  return (
    <header className="sticky top-0 z-20 flex h-14 shrink-0 items-center gap-2 border-b bg-background/85 px-3 backdrop-blur supports-[backdrop-filter]:bg-background/70 sm:px-4">
      <SidebarTrigger className="-ml-1 md:hidden" />
      <nav aria-label="Breadcrumb" className="flex min-w-0 flex-1 items-center gap-1 overflow-hidden">
        <span className="hidden min-w-0 items-center sm:inline-flex">
          <TeamCrumb />
        </span>
        <span className="flex min-w-0 items-center gap-1 overflow-x-auto [scrollbar-width:none] [&>*:first-child]:hidden sm:[&>*:first-child]:inline">
          {match ? <ResourceCrumbs match={match} /> : <PageCrumbs items={crumbs} />}
        </span>
        <TopBarTarget slot="status" className="ml-1 hidden shrink-0 items-center gap-2 lg:flex" />
      </nav>
      <TopBarTarget slot="actions" className="hidden shrink-0 items-center gap-2 lg:flex" />
    </header>
  );
}
