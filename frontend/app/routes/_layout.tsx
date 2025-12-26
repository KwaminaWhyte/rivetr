import { Link, Outlet, useLocation } from "react-router";
import type { Route } from "./+types/_layout";
import { AppSidebar } from "@/components/app-sidebar";
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "@/components/ui/breadcrumb";
import { Separator } from "@/components/ui/separator";
import {
  SidebarInset,
  SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar";

export async function loader({ request }: Route.LoaderArgs) {
  const { requireAuth } = await import("@/lib/session.server");
  await requireAuth(request);
  return null;
}

const routeTitles: Record<
  string,
  { parent?: string; parentUrl?: string; title: string }
> = {
  "/": { title: "Dashboard" },
  "/projects": { title: "Projects" },
  "/deployments": { title: "Deployments" },
  "/monitoring": { title: "Monitoring" },
  "/notifications": { title: "Notifications" },
  "/settings": { title: "Settings" },
  "/settings/git-providers": {
    parent: "Settings",
    parentUrl: "/settings",
    title: "Git Providers",
  },
  "/settings/ssh-keys": {
    parent: "Settings",
    parentUrl: "/settings",
    title: "SSH Keys",
  },
  "/settings/webhooks": {
    parent: "Settings",
    parentUrl: "/settings",
    title: "Webhooks",
  },
  "/settings/tokens": {
    parent: "Settings",
    parentUrl: "/settings",
    title: "API Tokens",
  },
};

function getBreadcrumb(pathname: string) {
  // Handle dynamic routes
  if (pathname.match(/^\/apps\/[^/]+\/settings$/)) {
    return { parent: "App Details", parentUrl: pathname.replace("/settings", ""), title: "Settings" };
  }
  if (pathname.match(/^\/apps\/[^/]+\/deployments$/)) {
    return { parent: "App Details", parentUrl: pathname.replace("/deployments", ""), title: "Deployments" };
  }
  if (pathname.match(/^\/apps\/[^/]+\/logs$/)) {
    return { parent: "App Details", parentUrl: pathname.replace("/logs", ""), title: "Logs" };
  }
  if (pathname.match(/^\/apps\/[^/]+\/terminal$/)) {
    return { parent: "App Details", parentUrl: pathname.replace("/terminal", ""), title: "Terminal" };
  }
  if (pathname.startsWith("/apps/")) {
    return { parent: "Projects", parentUrl: "/projects", title: "App Details" };
  }
  if (pathname.startsWith("/projects/") && pathname.includes("/apps/new")) {
    return {
      parent: "Projects",
      parentUrl: "/projects",
      title: "New Application",
    };
  }
  if (pathname.startsWith("/projects/")) {
    return {
      parent: "Projects",
      parentUrl: "/projects",
      title: "Project Details",
    };
  }
  return routeTitles[pathname] || { title: "Page" };
}

export default function DashboardLayout() {
  const location = useLocation();
  const breadcrumb = getBreadcrumb(location.pathname);

  return (
    <SidebarProvider>
      <AppSidebar />
      <SidebarInset>
        <header className="flex h-16 shrink-0 items-center gap-2 transition-[width,height] ease-linear group-has-data-[collapsible=icon]/sidebar-wrapper:h-12">
          <div className="flex items-center gap-2 px-4">
            <SidebarTrigger className="-ml-1" />
            <Separator
              orientation="vertical"
              className="mr-2 data-[orientation=vertical]:h-4"
            />
            <Breadcrumb>
              <BreadcrumbList>
                {breadcrumb.parent && breadcrumb.parentUrl && (
                  <>
                    <BreadcrumbItem className="hidden md:block">
                      <BreadcrumbLink asChild>
                        <Link to={breadcrumb.parentUrl}>
                          {breadcrumb.parent}
                        </Link>
                      </BreadcrumbLink>
                    </BreadcrumbItem>
                    <BreadcrumbSeparator className="hidden md:block" />
                  </>
                )}
                <BreadcrumbItem>
                  <BreadcrumbPage>{breadcrumb.title}</BreadcrumbPage>
                </BreadcrumbItem>
              </BreadcrumbList>
            </Breadcrumb>
          </div>
        </header>
        <div className="flex flex-1 flex-col gap-4 p-4 pt-0">
          <Outlet />
        </div>
      </SidebarInset>
    </SidebarProvider>
  );
}
