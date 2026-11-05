import { Link, useLocation } from "react-router";
import { ArrowLeft } from "lucide-react";

import {
  SidebarGroup,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from "@/components/ui/sidebar";
import { NavIconTile } from "@/components/nav-main";
import {
  isResourceLinkActive,
  RESOURCE_ICON,
  RESOURCE_KIND_LABEL,
  resourceSections,
  type ResourceMatch,
} from "@/lib/resource-nav";
import { useResourceInfo } from "@/lib/resource-queries";

/**
 * Contextual sidebar shown inside an app, database, service or server:
 * a back link, the resource name, then its grouped section links.
 */
export function NavResource({ match }: { match: ResourceMatch }) {
  const { pathname } = useLocation();
  const { isMobile, setOpenMobile } = useSidebar();
  const info = useResourceInfo(match);
  const sections = resourceSections(match, { dbType: info.variant });
  const KindIcon = RESOURCE_ICON[match.kind];
  const backHref =
    match.kind === "server" ? "/servers" : info.projectId ? `/projects/${info.projectId}` : "/";
  const backLabel = match.kind === "server" ? "Servers" : info.projectId ? "Project" : "Dashboard";
  const closeMobile = () => {
    if (isMobile) setOpenMobile(false);
  };

  return (
    <>
      <SidebarGroup className="pb-1">
        <SidebarMenu className="gap-0.5">
          <SidebarMenuItem>
            <SidebarMenuButton asChild tooltip={`Back to ${backLabel.toLowerCase()}`} className="h-8 text-muted-foreground">
              <Link to={backHref} onClick={closeMobile}>
                <ArrowLeft />
                <span>{backLabel}</span>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
          <SidebarMenuItem>
            <SidebarMenuButton asChild tooltip={info.name ?? RESOURCE_KIND_LABEL[match.kind]} className="h-11 gap-2.5 px-1.5 group-data-[collapsible=icon]:p-1!">
              <Link to={match.base} onClick={closeMobile}>
                <NavIconTile icon={KindIcon} active />
                <span className="grid min-w-0 leading-tight">
                  <span className="truncate text-sm font-medium">{info.name ?? "Loading"}</span>
                  <span className="truncate text-[11px] text-muted-foreground">
                    {RESOURCE_KIND_LABEL[match.kind]}
                  </span>
                </span>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarGroup>
      {sections.map((section, i) => (
        <SidebarGroup key={section.label ?? i} className="py-1">
          {section.label && (
            <SidebarGroupLabel className="h-6 text-[11px] font-medium text-muted-foreground/80">
              {section.label}
            </SidebarGroupLabel>
          )}
          <SidebarMenu className="gap-0.5">
            {section.items.map((item) => {
              const active = isResourceLinkActive(pathname, item.url, item.exact);
              return (
                <SidebarMenuItem key={item.url}>
                  <SidebarMenuButton
                    asChild
                    tooltip={item.title}
                    isActive={active}
                    className="h-8 gap-2.5 px-1.5 group-data-[collapsible=icon]:p-1!"
                  >
                    <Link to={item.url} onClick={closeMobile}>
                      <NavIconTile icon={item.icon} active={active} />
                      <span>{item.title}</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              );
            })}
          </SidebarMenu>
        </SidebarGroup>
      ))}
    </>
  );
}
