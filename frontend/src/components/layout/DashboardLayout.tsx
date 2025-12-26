import { Link, Outlet, useLocation } from "react-router";
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

const routeTitles: Record<string, { parent?: string; parentUrl?: string; title: string }> = {
  "/": { title: "Dashboard" },
  "/apps": { title: "Applications" },
  "/apps/new": { parent: "Applications", parentUrl: "/apps", title: "New Application" },
  "/deployments": { title: "Deployments" },
  "/settings": { title: "Settings" },
  "/settings/webhooks": { parent: "Settings", parentUrl: "/settings", title: "Webhooks" },
  "/settings/tokens": { parent: "Settings", parentUrl: "/settings", title: "API Tokens" },
};

function getBreadcrumb(pathname: string) {
  // Handle dynamic app detail routes
  if (pathname.startsWith("/apps/") && pathname !== "/apps/new") {
    return { parent: "Applications", parentUrl: "/apps", title: "App Details" };
  }
  return routeTitles[pathname] || { title: "Page" };
}

export function DashboardLayout() {
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
                        <Link to={breadcrumb.parentUrl}>{breadcrumb.parent}</Link>
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
