import { isRouteErrorResponse, Link, Outlet, useLocation, useRouteError } from "react-router";
import type { ReactNode } from "react";
import { useState } from "react";
import { AppSidebar } from "@/components/app-sidebar";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";
import { CommandPalette } from "@/components/command-palette";
import { SectionTabs } from "@/components/section-tabs";
import { AreaShell } from "@/components/area-shell";
import { TopBar } from "@/components/shell/top-bar";
import { TopBarSlotProvider } from "@/components/shell/top-bar-slot";
import { RivetrLogo } from "@/components/rivetr-logo";
import { useRequireAuth } from "@/lib/auth";
import { BreadcrumbProvider, type BreadcrumbItem } from "@/lib/breadcrumb-context";
import { EmptyState } from "@/components/page";
import { Button } from "@/components/ui/button";
import { TeamProvider } from "@/lib/team-context";
import { TwoFactorBanner } from "@/components/teams/two-factor-banner";
import { cn } from "@/lib/utils";

/** Pages that use the whole content width (canvas views). */
function isFullBleed(pathname: string, search: string): boolean {
  if (pathname === "/") return true;
  return pathname === "/projects" && new URLSearchParams(search).get("view") === "canvas";
}

function DashboardLayoutInner({
  children,
  crumbs,
}: {
  children?: ReactNode;
  crumbs?: BreadcrumbItem[];
}) {
  const location = useLocation();
  const { isLoading, isAuthenticated } = useRequireAuth();
  const [searchOpen, setSearchOpen] = useState(false);

  if (isLoading) {
    return (
      <div className="flex h-screen flex-col items-center justify-center gap-3 bg-background">
        <RivetrLogo decorative className="size-9 animate-pulse" />
        <span className="text-sm text-muted-foreground">Loading</span>
      </div>
    );
  }

  if (!isAuthenticated) {
    return null;
  }

  return (
    <SidebarProvider>
      <TopBarSlotProvider>
        <AppSidebar onOpenSearch={() => setSearchOpen(true)} />
        <CommandPalette open={searchOpen} onOpenChange={setSearchOpen} />
        <SidebarInset className="min-w-0 bg-background">
          <TopBar crumbs={crumbs} />
          <div
            className={cn(
              "mx-auto flex w-full flex-1 flex-col gap-5 px-4 py-5 sm:px-6 lg:py-6",
              isFullBleed(location.pathname, location.search) ? "max-w-[1600px]" : "max-w-7xl"
            )}
          >
            <TwoFactorBanner />
            <SectionTabs />
            <AreaShell>{children ?? <Outlet />}</AreaShell>
          </div>
        </SidebarInset>
      </TopBarSlotProvider>
    </SidebarProvider>
  );
}

export default function DashboardLayout() {
  return (
    <TeamProvider>
      <BreadcrumbProvider>
        <DashboardLayoutInner />
      </BreadcrumbProvider>
    </TeamProvider>
  );
}

/**
 * Errors thrown by any dashboard page render inside the shell, so the
 * sidebar and breadcrumbs stay and the user can navigate away.
 */
export function ErrorBoundary() {
  const error = useRouteError();
  const notFound = isRouteErrorResponse(error) && error.status === 404;
  const title = notFound ? "Page not found" : "Something went wrong";
  const details = isRouteErrorResponse(error)
    ? error.statusText || `Error ${error.status}`
    : error instanceof Error
      ? error.message
      : "An unexpected error occurred.";

  return (
    <TeamProvider>
      <BreadcrumbProvider>
        <DashboardLayoutInner crumbs={[{ label: title }]}>
          <EmptyState
            bordered
            title={title}
            description={details}
            action={
              <div className="flex gap-2">
                <Button variant="outline" onClick={() => window.location.reload()}>
                  Reload
                </Button>
                <Button asChild>
                  <Link to="/">Back to dashboard</Link>
                </Button>
              </div>
            }
          />
        </DashboardLayoutInner>
      </BreadcrumbProvider>
    </TeamProvider>
  );
}
