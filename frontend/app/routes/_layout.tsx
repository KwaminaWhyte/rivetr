import { Outlet, useLocation } from "react-router";
import { useState } from "react";
import { AppSidebar } from "@/components/app-sidebar";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";
import { CommandPalette } from "@/components/command-palette";
import { SectionTabs } from "@/components/section-tabs";
import { SettingsShell } from "@/components/settings-nav";
import { TopBar } from "@/components/shell/top-bar";
import { TopBarSlotProvider } from "@/components/shell/top-bar-slot";
import { RivetrLogo } from "@/components/rivetr-logo";
import { useRequireAuth } from "@/lib/auth";
import { BreadcrumbProvider } from "@/lib/breadcrumb-context";
import { TeamProvider } from "@/lib/team-context";
import { cn } from "@/lib/utils";

/** Pages that use the whole content width (canvas views). */
function isFullBleed(pathname: string, search: string): boolean {
  if (pathname === "/") return true;
  return pathname === "/projects" && new URLSearchParams(search).get("view") === "canvas";
}

function DashboardLayoutInner() {
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
          <TopBar />
          <div
            className={cn(
              "mx-auto flex w-full flex-1 flex-col gap-5 px-4 py-5 sm:px-6 lg:py-6",
              isFullBleed(location.pathname, location.search) ? "max-w-[1600px]" : "max-w-7xl"
            )}
          >
            <SectionTabs />
            <SettingsShell>
              <Outlet />
            </SettingsShell>
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
