import * as React from "react";
import { useQuery } from "@tanstack/react-query";
import {
  LayoutDashboard,
  FolderKanban,
  Settings,
  BarChart3,
  Server,
  Cpu,
  Key,
  Network,
  Users,
  GitBranch,
  Webhook,
  Activity,
  KeyRound,
  Globe,
  LayoutTemplate,
  ExternalLink,
  MessageCircle,
  ShieldCheck,
} from "lucide-react";

import { NavMain, type NavMainItem } from "@/components/nav-main";
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
import { useWhiteLabel } from "@/lib/white-label-context";
import { api } from "@/lib/api";
import type { UpdateStatus } from "@/types/api";

const navPlatform: NavMainItem[] = [
  { title: "Dashboard", url: "/", icon: LayoutDashboard },
  { title: "Projects", url: "/projects", icon: FolderKanban },
  { title: "Monitoring", url: "/monitoring", icon: BarChart3 },
  { title: "Templates", url: "/templates", icon: LayoutTemplate },
];

const navInfrastructure: NavMainItem[] = [
  { title: "Servers", url: "/servers", icon: Server },
  { title: "Build Servers", url: "/build-servers", icon: Cpu },
  { title: "SSH Keys", url: "/ssh-keys", icon: Key },
  { title: "Docker Swarm", url: "/swarm", icon: Network },
  { title: "Tunnels", url: "/tunnels", icon: Globe },
];

const navAccess: NavMainItem[] = [
  { title: "Teams", url: "/teams", icon: Users },
  { title: "Git Integrations", url: "/git-providers", icon: GitBranch },
  { title: "Webhooks", url: "/webhooks", icon: Webhook },
  { title: "Webhook Events", url: "/webhook-events", icon: Activity },
  { title: "API Tokens", url: "/tokens", icon: KeyRound },
];

const navSettings: NavMainItem[] = [
  {
    title: "Settings",
    url: "/settings",
    icon: Settings,
    items: [
      { title: "General", url: "/settings" },
      { title: "White Label", url: "/settings/white-label" },
      { title: "Security", url: "/settings/security" },
      { title: "Auto Updates", url: "/settings/auto-update" },
      { title: "Authentication", url: "/settings/oauth" },
      { title: "SSO / OIDC", url: "/settings/sso" },
      { title: "Backup & Restore", url: "/settings/backup" },
      { title: "S3 Storage", url: "/settings/s3" },
      { title: "Alert Defaults", url: "/settings/alert-defaults" },
      { title: "Notifications", url: "/settings/notifications" },
      { title: "Audit Log", url: "/settings/audit" },
      { title: "CA Certificates", url: "/settings/ca-certificates" },
      { title: "Destinations", url: "/settings/destinations" },
    ],
  },
];

function SidebarFooterInfo() {
  const { state } = useSidebar();
  const isCollapsed = state === "collapsed";

  const { data: versionInfo } = useQuery<UpdateStatus | null>({
    queryKey: ["version-info"],
    queryFn: () => api.getVersionInfo(),
    staleTime: 10 * 60 * 1000,
    retry: false,
  });

  const version = versionInfo?.current_version ?? "v0.10.8";

  if (isCollapsed) return null;

  return (
    <div className="px-2 py-1">
      <SidebarMenu>
        <SidebarMenuItem>
          <SidebarMenuButton asChild size="sm" className="h-7 text-xs text-muted-foreground hover:text-foreground">
            <a
              href={`https://github.com/KwaminaWhyte/rivetr/releases/tag/${version}`}
              target="_blank"
              rel="noopener noreferrer"
              title={`Rivetr ${version} — view release notes`}
            >
              <span className="font-mono">{version}</span>
              <ExternalLink className="ml-auto h-3 w-3 opacity-50" />
            </a>
          </SidebarMenuButton>
        </SidebarMenuItem>
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

export function AppSidebar({ ...props }: React.ComponentProps<typeof Sidebar>) {
  const { teams, currentTeamId, setCurrentTeamId } = useTeamContext();
  const { config: wl } = useWhiteLabel();

  return (
    <Sidebar collapsible="icon" {...props}>
      <SidebarHeader>
        {/* Branding header: custom logo / app name if configured */}
        {(wl?.logo_url || (wl?.app_name && wl.app_name !== "Rivetr")) && (
          <div className="flex items-center gap-2 px-2 py-1 mb-1">
            {wl.logo_url ? (
              <img
                src={wl.logo_url}
                alt={wl.app_name}
                className="h-6 w-auto object-contain"
              />
            ) : null}
            <span className="font-semibold text-sm truncate">{wl.app_name}</span>
          </div>
        )}
        <TeamSwitcher
          teams={teams}
          currentTeamId={currentTeamId}
          onTeamChange={setCurrentTeamId}
        />
      </SidebarHeader>
      <SidebarContent>
        <NavMain items={navPlatform} label="Platform" />
        <NavMain items={navInfrastructure} label="Infrastructure" />
        <NavMain items={navAccess} label="Access" />
        <NavMain items={navSettings} />
      </SidebarContent>
      <SidebarFooter>
        <SidebarFooterInfo />
        <NavUser />
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  );
}
