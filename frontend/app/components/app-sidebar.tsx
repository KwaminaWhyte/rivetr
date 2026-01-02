import * as React from "react";
import {
  LayoutDashboard,
  FolderKanban,
  Settings,
  BarChart3,
  Bell,
} from "lucide-react";

import { NavMain, type NavMainItem } from "@/components/nav-main";
import { NavUser } from "@/components/nav-user";
import { NavBrand } from "@/components/nav-brand";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarRail,
} from "@/components/ui/sidebar";

const navMain: NavMainItem[] = [
  {
    title: "Dashboard",
    url: "/",
    icon: LayoutDashboard,
  },
  {
    title: "Projects",
    url: "/projects",
    icon: FolderKanban,
  },
  {
    title: "Monitoring",
    url: "/monitoring",
    icon: BarChart3,
  },
  {
    title: "Notifications",
    url: "/notifications",
    icon: Bell,
  },
  {
    title: "Settings",
    url: "/settings",
    icon: Settings,
    items: [
      { title: "General", url: "/settings" },
      { title: "Teams", url: "/settings/teams" },
      { title: "Notifications", url: "/settings/notifications" },
      { title: "Git Integrations", url: "/settings/git-providers" },
      { title: "SSH Keys", url: "/settings/ssh-keys" },
      { title: "Webhooks", url: "/settings/webhooks" },
      { title: "API Tokens", url: "/settings/tokens" },
      { title: "Audit Log", url: "/settings/audit" },
    ],
  },
];

export function AppSidebar({ ...props }: React.ComponentProps<typeof Sidebar>) {
  return (
    <Sidebar collapsible="icon" {...props}>
      <SidebarHeader>
        <NavBrand />
      </SidebarHeader>
      <SidebarContent>
        <NavMain items={navMain} />
      </SidebarContent>
      <SidebarFooter>
        <NavUser />
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  );
}
