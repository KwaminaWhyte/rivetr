import { useEffect } from "react";
import { Link, NavLink, Outlet, useParams } from "react-router";
import { useQuery } from "@tanstack/react-query";
import {
  ArrowLeft,
  Container,
  FolderOpen,
  Info,
  Layers,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { useBreadcrumb } from "@/lib/breadcrumb-context";
import { serverOpsApi, LOCALHOST_SERVER_ID } from "@/lib/api/server-ops";
import type { ServerContext } from "@/components/servers/server-context";
import { ServerStatusPill } from "@/components/servers/server-status-pill";
import { cn } from "@/lib/utils";

export function meta() {
  return [
    { title: "Server - Rivetr" },
    { name: "description", content: "Server images, Docker cleanup and patching" },
  ];
}

interface NavItem {
  label: string;
  path: string;
  icon: typeof Info;
  remoteOnly?: boolean;
}

const NAV_GROUPS: { title?: string; items: NavItem[] }[] = [
  { items: [{ label: "General", path: "", icon: Info }] },
  {
    title: "Docker",
    items: [
      { label: "Images", path: "/images", icon: Layers },
      { label: "Docker Cleanup", path: "/docker-cleanup", icon: Sparkles },
    ],
  },
  {
    title: "Host",
    items: [
      { label: "Security", path: "/security", icon: ShieldCheck },
      { label: "Files", path: "/files", icon: FolderOpen, remoteOnly: true },
    ],
  },
];

export default function ServerLayout() {
  const { id = "" } = useParams<{ id: string }>();
  const isLocal = id === LOCALHOST_SERVER_ID;
  const { setItems } = useBreadcrumb();

  const { data: summaries } = useQuery({
    queryKey: ["server-ops-summary"],
    queryFn: () => serverOpsApi.summary(),
  });
  const summary = summaries?.find((s) => s.server_id === id);
  const name = summary?.name ?? (isLocal ? "localhost" : "Server");

  useEffect(() => {
    setItems([{ label: "Servers", href: "/servers" }, { label: name }]);
  }, [name, setItems]);

  const basePath = `/servers/${id}`;

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="space-y-1">
          <Link
            to="/servers"
            className="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground md:hidden"
          >
            <ArrowLeft className="h-3 w-3" />
            Servers
          </Link>
          <div className="flex items-center gap-3">
            <Container className="h-6 w-6 text-muted-foreground" />
            <h1 className="text-2xl font-semibold tracking-tight">{name}</h1>
            {summary && <ServerStatusPill status={summary.status} />}
          </div>
          <p className="text-sm text-muted-foreground">
            {isLocal ? "The machine this Rivetr instance runs on" : summary?.host}
          </p>
        </div>
      </div>

      <div className="flex flex-col gap-6 md:flex-row">
        <nav className="shrink-0 md:hidden" aria-label="Server sections">
          <div className="flex gap-1 overflow-x-auto md:flex-col md:overflow-visible">
            {NAV_GROUPS.map((group, gi) => (
              <div key={gi} className="flex gap-1 md:flex-col md:pb-3">
                {group.title && (
                  <p className="hidden px-2 pb-1 text-xs font-medium uppercase tracking-wide text-muted-foreground md:block">
                    {group.title}
                  </p>
                )}
                {group.items
                  .filter((item) => !(item.remoteOnly && isLocal))
                  .map((item) => (
                    <NavLink
                      key={item.path}
                      to={`${basePath}${item.path}`}
                      end={item.path === ""}
                      className={({ isActive }) =>
                        cn(
                          "flex items-center gap-2 whitespace-nowrap rounded-md px-2 py-1.5 text-sm transition-colors",
                          isActive
                            ? "bg-muted font-medium text-foreground"
                            : "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
                        )
                      }
                    >
                      <item.icon className="h-4 w-4" />
                      {item.label}
                    </NavLink>
                  ))}
              </div>
            ))}
          </div>
        </nav>
        <div className="min-w-0 flex-1">
          <Outlet context={{ serverId: id, isLocal, name, summary } satisfies ServerContext} />
        </div>
      </div>
    </div>
  );
}
