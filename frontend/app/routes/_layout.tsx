import { Link, Outlet, useLocation } from "react-router";
import { Fragment, useEffect } from "react";
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
import { useRequireAuth } from "@/lib/auth";
import { BreadcrumbProvider, useBreadcrumb, type BreadcrumbItem as BreadcrumbItemType } from "@/lib/breadcrumb-context";

// Static route titles as fallback
const routeTitles: Record<string, BreadcrumbItemType[]> = {
  "/": [{ label: "Dashboard" }],
  "/projects": [{ label: "Projects" }],
  "/deployments": [{ label: "Deployments" }],
  "/monitoring": [{ label: "Monitoring" }],
  "/notifications": [{ label: "Notifications" }],
  "/settings": [{ label: "Settings" }],
  "/settings/git-providers": [
    { label: "Settings", href: "/settings" },
    { label: "Git Providers" },
  ],
  "/settings/ssh-keys": [
    { label: "Settings", href: "/settings" },
    { label: "SSH Keys" },
  ],
  "/settings/webhooks": [
    { label: "Settings", href: "/settings" },
    { label: "Webhooks" },
  ],
  "/settings/tokens": [
    { label: "Settings", href: "/settings" },
    { label: "API Tokens" },
  ],
  "/settings/notifications": [
    { label: "Settings", href: "/settings" },
    { label: "Notifications" },
  ],
  "/settings/audit": [
    { label: "Settings", href: "/settings" },
    { label: "Audit Log" },
  ],
};

function getDefaultBreadcrumbs(pathname: string): BreadcrumbItemType[] {
  // Check static routes first
  if (routeTitles[pathname]) {
    return routeTitles[pathname];
  }

  // Handle dynamic routes with defaults
  if (pathname.match(/^\/apps\/[^/]+\/settings$/)) {
    return [
      { label: "Projects", href: "/projects" },
      { label: "Apps" },
      { label: "Settings" },
    ];
  }
  if (pathname.match(/^\/apps\/[^/]+\/deployments$/)) {
    return [
      { label: "Projects", href: "/projects" },
      { label: "Apps" },
      { label: "Deployments" },
    ];
  }
  if (pathname.match(/^\/apps\/[^/]+\/network$/)) {
    return [
      { label: "Projects", href: "/projects" },
      { label: "Apps" },
      { label: "Network" },
    ];
  }
  if (pathname.match(/^\/apps\/[^/]+\/logs$/)) {
    return [
      { label: "Projects", href: "/projects" },
      { label: "Apps" },
      { label: "Logs" },
    ];
  }
  if (pathname.match(/^\/apps\/[^/]+\/terminal$/)) {
    return [
      { label: "Projects", href: "/projects" },
      { label: "Apps" },
      { label: "Terminal" },
    ];
  }
  if (pathname.match(/^\/apps\/[^/]+$/)) {
    return [
      { label: "Projects", href: "/projects" },
      { label: "Apps" },
    ];
  }
  if (pathname.match(/^\/databases\/[^/]+/)) {
    return [
      { label: "Projects", href: "/projects" },
      { label: "Databases" },
    ];
  }
  if (pathname.match(/^\/services\/[^/]+/)) {
    return [
      { label: "Projects", href: "/projects" },
      { label: "Services" },
    ];
  }
  if (pathname.startsWith("/projects/") && pathname.includes("/apps/new")) {
    return [
      { label: "Projects", href: "/projects" },
      { label: "New Application" },
    ];
  }
  if (pathname.match(/^\/projects\/[^/]+$/)) {
    return [
      { label: "Projects", href: "/projects" },
      { label: "Project" },
    ];
  }

  return [{ label: "Page" }];
}

// Inner component that uses the breadcrumb context
function DashboardLayoutInner() {
  const location = useLocation();
  const { items: contextItems, setItems } = useBreadcrumb();
  const { isLoading, isAuthenticated } = useRequireAuth();

  // Set default breadcrumbs based on route when no context items are set
  useEffect(() => {
    if (contextItems.length === 0) {
      setItems(getDefaultBreadcrumbs(location.pathname));
    }
  }, [location.pathname, contextItems.length, setItems]);

  // Reset breadcrumbs when route changes
  useEffect(() => {
    setItems(getDefaultBreadcrumbs(location.pathname));
  }, [location.pathname, setItems]);

  const breadcrumbItems = contextItems.length > 0 ? contextItems : getDefaultBreadcrumbs(location.pathname);

  // Show loading state while checking auth
  if (isLoading) {
    return (
      <div className="flex h-screen items-center justify-center">
        <div className="animate-pulse text-muted-foreground">Loading...</div>
      </div>
    );
  }

  // Don't render content if not authenticated (redirect will happen)
  if (!isAuthenticated) {
    return null;
  }

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
                {breadcrumbItems.map((item, index) => (
                  <Fragment key={index}>
                    {index > 0 && <BreadcrumbSeparator className="hidden md:block" />}
                    <BreadcrumbItem className={index < breadcrumbItems.length - 1 ? "hidden md:block" : ""}>
                      {item.href && index < breadcrumbItems.length - 1 ? (
                        <BreadcrumbLink asChild>
                          <Link to={item.href}>{item.label}</Link>
                        </BreadcrumbLink>
                      ) : (
                        <BreadcrumbPage>{item.label}</BreadcrumbPage>
                      )}
                    </BreadcrumbItem>
                  </Fragment>
                ))}
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

export default function DashboardLayout() {
  return (
    <BreadcrumbProvider>
      <DashboardLayoutInner />
    </BreadcrumbProvider>
  );
}
