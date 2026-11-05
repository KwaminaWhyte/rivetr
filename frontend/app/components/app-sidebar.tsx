import * as React from "react";
import { Link, useLocation } from "react-router";
import { Search } from "lucide-react";

import { NavMain } from "@/components/nav-main";
import { NavResource } from "@/components/nav-resource";
import { NavUser, useVersionInfo } from "@/components/nav-user";
import { RivetrLogo } from "@/components/rivetr-logo";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
} from "@/components/ui/sidebar";
import { matchResource } from "@/lib/resource-nav";
import { useWhiteLabel } from "@/lib/white-label-context";

function Brand() {
  const { config } = useWhiteLabel();
  const { data: versionInfo } = useVersionInfo();
  const version = versionInfo?.current_version;
  return (
    <SidebarMenu>
      <SidebarMenuItem>
        <SidebarMenuButton asChild size="lg" tooltip={config?.app_name || "Rivetr"} className="h-11 gap-2.5 px-1.5 hover:bg-transparent">
          <Link to="/">
            <RivetrLogo decorative className="size-7 rounded-md" />
            <span className="flex min-w-0 items-baseline gap-1.5">
              <span className="truncate text-[15px] font-semibold tracking-tight">
                {config?.app_name || "Rivetr"}
              </span>
              {version && (
                <span className="truncate font-mono text-[10px] text-muted-foreground">{version}</span>
              )}
            </span>
          </Link>
        </SidebarMenuButton>
      </SidebarMenuItem>
    </SidebarMenu>
  );
}

function SearchButton({ onOpenSearch }: { onOpenSearch: () => void }) {
  return (
    <SidebarMenu>
      <SidebarMenuItem>
        <SidebarMenuButton
          tooltip="Search (Cmd+K)"
          onClick={onOpenSearch}
          className="h-8 border bg-field text-muted-foreground shadow-none hover:text-foreground group-data-[collapsible=icon]:border-0 group-data-[collapsible=icon]:bg-transparent"
        >
          <Search />
          <span>Search</span>
          <kbd className="ml-auto rounded border bg-card px-1.5 font-mono text-[10px] leading-4 text-muted-foreground">
            {"⌘"}K
          </kbd>
        </SidebarMenuButton>
      </SidebarMenuItem>
    </SidebarMenu>
  );
}

/**
 * Dashboard sidebar. Shows the grouped global nav, or a contextual section
 * list when inside an app, database, service or server. Becomes a sheet on
 * mobile (handled by the Sidebar primitive).
 */
export function AppSidebar({
  onOpenSearch,
  ...props
}: React.ComponentProps<typeof Sidebar> & { onOpenSearch?: () => void }) {
  const { pathname } = useLocation();
  const resource = matchResource(pathname);

  return (
    <Sidebar collapsible="icon" {...props}>
      <SidebarHeader className="gap-2 pb-1">
        <Brand />
        {onOpenSearch && <SearchButton onOpenSearch={onOpenSearch} />}
      </SidebarHeader>
      <SidebarContent className="gap-0">
        {resource ? <NavResource match={resource} /> : <NavMain />}
      </SidebarContent>
      <SidebarFooter className="border-t">
        <NavUser />
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  );
}
