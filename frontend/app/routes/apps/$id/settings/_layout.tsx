import { useOutletContext, Outlet, Link, useLocation, useParams } from "react-router";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Bell, Shield } from "lucide-react";
import type { App } from "@/types/api";

export function meta() {
  return [
    { title: "App Settings - Rivetr" },
    { name: "description", content: "Configure application settings, environment variables, and resources" },
  ];
}

interface ParentContext {
  app: App;
}

const SETTINGS_TABS = [
  { id: "general", label: "General", path: "" },
  { id: "build", label: "Build", path: "/build" },
  { id: "network", label: "Network", path: "/network" },
  { id: "storage", label: "Storage", path: "/storage" },
  { id: "alerts", label: "Alerts", path: "/alerts", icon: Bell },
  { id: "security", label: "Security", path: "/security" },
  { id: "deployment", label: "Deploy", path: "/deployment", icon: Shield },
  { id: "replicas", label: "Replicas", path: "/replicas" },
  { id: "snapshots", label: "Snapshots", path: "/snapshots" },
  { id: "sharing", label: "Sharing", path: "/sharing" },
  { id: "docker", label: "Docker", path: "/docker" },
  { id: "patches", label: "Patches", path: "/patches" },
];

export default function AppSettingsLayout() {
  const { app } = useOutletContext<ParentContext>();
  const { id } = useParams();
  const location = useLocation();
  const basePath = `/apps/${id}/settings`;

  const activeTab =
    SETTINGS_TABS.find((tab) => {
      if (tab.path === "") {
        return location.pathname === basePath || location.pathname === basePath + "/";
      }
      return location.pathname.startsWith(basePath + tab.path);
    })?.id || "general";

  return (
    <div className="space-y-6">
      <Tabs value={activeTab} className="w-full">
        <TabsList className="grid w-full grid-cols-12">
          {SETTINGS_TABS.map((tab) => (
            <TabsTrigger key={tab.id} value={tab.id} asChild>
              <Link to={`${basePath}${tab.path}`} className="gap-1">
                {tab.icon && <tab.icon className="h-4 w-4" />}
                {tab.label}
              </Link>
            </TabsTrigger>
          ))}
        </TabsList>
      </Tabs>
      <Outlet context={{ app }} />
    </div>
  );
}
