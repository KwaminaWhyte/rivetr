import { Link } from "react-router";
import { useQuery } from "@tanstack/react-query";
import {
  MoreHorizontal,
  Package,
  Rocket,
  Trash2,
  ExternalLink,
  Plus,
} from "lucide-react";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  SidebarGroup,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuAction,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSkeleton,
  useSidebar,
} from "@/components/ui/sidebar";
import { api } from "@/lib/api";
import { useTeamContext } from "@/lib/team-context";
import type { App } from "@/types/api";

export function NavApps() {
  const { isMobile } = useSidebar();
  const { currentTeamId } = useTeamContext();

  const { data: apps = [], isLoading } = useQuery<App[]>({
    queryKey: ["apps", currentTeamId],
    queryFn: () => api.getApps({ teamId: currentTeamId ?? undefined }),
    enabled: currentTeamId !== null,
  });

  // Show only 5 most recent apps
  const recentApps = apps.slice(0, 5);

  return (
    <SidebarGroup className="group-data-[collapsible=icon]:hidden">
      <SidebarGroupLabel>Recent Apps</SidebarGroupLabel>
      <SidebarMenu>
        {isLoading ? (
          <>
            <SidebarMenuSkeleton />
            <SidebarMenuSkeleton />
            <SidebarMenuSkeleton />
          </>
        ) : recentApps.length === 0 ? (
          <SidebarMenuItem>
            <SidebarMenuButton asChild>
              <Link to="/apps/new" className="text-muted-foreground">
                <Plus className="size-4" />
                <span>Create your first app</span>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        ) : (
          <>
            {recentApps.map((app) => (
              <SidebarMenuItem key={app.id}>
                <SidebarMenuButton asChild>
                  <Link to={`/apps/${app.id}`}>
                    <Package className="size-4" />
                    <span>{app.name}</span>
                  </Link>
                </SidebarMenuButton>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <SidebarMenuAction showOnHover>
                      <MoreHorizontal />
                      <span className="sr-only">More</span>
                    </SidebarMenuAction>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent
                    className="w-48 rounded-lg"
                    side={isMobile ? "bottom" : "right"}
                    align={isMobile ? "end" : "start"}
                  >
                    <DropdownMenuItem asChild>
                      <Link to={`/apps/${app.id}`}>
                        <Package className="mr-2 h-4 w-4 text-muted-foreground" />
                        <span>View Details</span>
                      </Link>
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onClick={() => api.triggerDeploy(app.id)}
                    >
                      <Rocket className="mr-2 h-4 w-4 text-muted-foreground" />
                      <span>Deploy</span>
                    </DropdownMenuItem>
                    {app.domain && (
                      <DropdownMenuItem asChild>
                        <a
                          href={`http://${app.domain}`}
                          target="_blank"
                          rel="noopener noreferrer"
                        >
                          <ExternalLink className="mr-2 h-4 w-4 text-muted-foreground" />
                          <span>Open Site</span>
                        </a>
                      </DropdownMenuItem>
                    )}
                    <DropdownMenuSeparator />
                    <DropdownMenuItem className="text-destructive">
                      <Trash2 className="mr-2 h-4 w-4" />
                      <span>Delete</span>
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </SidebarMenuItem>
            ))}
            {apps.length > 5 && (
              <SidebarMenuItem>
                <SidebarMenuButton asChild className="text-muted-foreground">
                  <Link to="/apps">
                    <MoreHorizontal className="size-4" />
                    <span>View all ({apps.length})</span>
                  </Link>
                </SidebarMenuButton>
              </SidebarMenuItem>
            )}
          </>
        )}
      </SidebarMenu>
    </SidebarGroup>
  );
}
