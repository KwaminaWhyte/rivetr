import * as React from "react";
import {
  LayoutDashboard,
  Package,
  Settings,
  Activity,
} from "lucide-react";

import { NavMain, type NavMainItem } from "@/components/nav-main";
import { NavApps } from "@/components/nav-apps";
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
    title: "Applications",
    url: "/apps",
    icon: Package,
    items: [
      { title: "All Applications", url: "/apps" },
      { title: "New Application", url: "/apps/new" },
    ],
  },
  {
    title: "Deployments",
    url: "/deployments",
    icon: Activity,
  },
  {
    title: "Settings",
    url: "/settings",
    icon: Settings,
    items: [
      { title: "General", url: "/settings" },
      { title: "Webhooks", url: "/settings/webhooks" },
      { title: "API Tokens", url: "/settings/tokens" },
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
        <NavApps />
      </SidebarContent>
      <SidebarFooter>
        <NavUser />
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  );
}
