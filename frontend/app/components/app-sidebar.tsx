import * as React from "react";
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
  Paintbrush,
  Globe,
  LayoutTemplate,
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
} from "@/components/ui/sidebar";
import { useTeamContext } from "@/lib/team-context";
import { useWhiteLabel } from "@/lib/white-label-context";

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
    ],
  },
];

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
        <NavUser />
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  );
}
