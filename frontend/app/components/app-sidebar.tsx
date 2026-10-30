import * as React from "react";
import { useQuery } from "@tanstack/react-query";
import { ExternalLink, MessageCircle, Search } from "lucide-react";

import { NavMain } from "@/components/nav-main";
import { NavUser } from "@/components/nav-user";
import { TeamSwitcher } from "@/components/team-switcher";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarRail,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
  useSidebar,
} from "@/components/ui/sidebar";
import { useTeamContext } from "@/lib/team-context";
import { api } from "@/lib/api";
import { primaryNav } from "@/lib/nav";
import type { UpdateStatus } from "@/types/api";

function SidebarFooterInfo() {
  const { state } = useSidebar();
  const isCollapsed = state === "collapsed";

  const { data: versionInfo } = useQuery<UpdateStatus | null>({
    queryKey: ["version-info"],
    queryFn: () => api.getVersionInfo(),
    staleTime: 10 * 60 * 1000,
    retry: false,
  });

  const version = versionInfo?.current_version;

  if (isCollapsed) return null;

  return (
    <div className="px-2 py-1">
      <SidebarMenu>
        {version && (
          <SidebarMenuItem>
            <SidebarMenuButton asChild size="sm" className="h-7 text-xs text-muted-foreground hover:text-foreground">
              <a
                href={`https://github.com/KwaminaWhyte/rivetr/releases/tag/${version}`}
                target="_blank"
                rel="noopener noreferrer"
                title={`Rivetr ${version}: view release notes`}
              >
                <span className="font-mono">{version}</span>
                <ExternalLink className="ml-auto h-3 w-3 opacity-50" />
              </a>
            </SidebarMenuButton>
          </SidebarMenuItem>
        )}
        <SidebarMenuItem>
          <SidebarMenuButton asChild size="sm" className="h-7 text-xs text-muted-foreground hover:text-foreground">
            <a
              href="https://github.com/KwaminaWhyte/rivetr/issues/new"
              target="_blank"
              rel="noopener noreferrer"
              title="Open a GitHub issue or feature request"
            >
              <MessageCircle className="h-3 w-3" />
              <span>Feedback</span>
              <ExternalLink className="ml-auto h-3 w-3 opacity-50" />
            </a>
          </SidebarMenuButton>
        </SidebarMenuItem>
      </SidebarMenu>
    </div>
  );
}

export function AppSidebar({
  onOpenSearch,
  ...props
}: React.ComponentProps<typeof Sidebar> & { onOpenSearch?: () => void }) {
  const { teams, currentTeamId, setCurrentTeamId } = useTeamContext();

  return (
    <Sidebar collapsible="icon" {...props}>
      <SidebarHeader>
        <TeamSwitcher
          teams={teams}
          currentTeamId={currentTeamId}
          onTeamChange={setCurrentTeamId}
        />
        {onOpenSearch && (
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton
                size="sm"
                tooltip="Search (Cmd+K or Ctrl+K)"
                onClick={onOpenSearch}
                className="text-muted-foreground hover:text-foreground"
              >
                <Search />
                <span>Search</span>
                <kbd className="ml-auto rounded border bg-muted px-1.5 font-mono text-[10px] text-muted-foreground">
                  {"\u2318"}K
                </kbd>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        )}
      </SidebarHeader>
      <SidebarContent>
        <NavMain items={primaryNav} />
      </SidebarContent>
      <SidebarFooter>
        <SidebarFooterInfo />
        <NavUser />
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  );
}
