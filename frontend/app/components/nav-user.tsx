import { Link } from "react-router";
import { useQuery } from "@tanstack/react-query";
import {
  BookOpen,
  ChevronsUpDown,
  LogOut,
  MessageCircle,
  Monitor,
  Moon,
  Palette,
  PanelLeftClose,
  PanelLeftOpen,
  Sparkles,
  Sun,
  UserRound,
} from "lucide-react";

import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuPortal,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from "@/components/ui/sidebar";
import { useTheme } from "@/components/providers/theme-provider";
import { api } from "@/lib/api";
import { getCurrentUser } from "@/lib/auth";
import { cn } from "@/lib/utils";
import type { UpdateStatus } from "@/types/api";

const REPO = "https://github.com/KwaminaWhyte/rivetr";

function getInitials(name: string): string {
  return name
    .split(" ")
    .map((n) => n[0])
    .join("")
    .toUpperCase()
    .slice(0, 2);
}

/** Version info shared with the sidebar header (same query key). */
export function useVersionInfo() {
  return useQuery<UpdateStatus | null>({
    queryKey: ["version-info"],
    queryFn: () => api.getVersionInfo(),
    staleTime: 10 * 60 * 1000,
    retry: false,
  });
}

/** User button at the bottom of the sidebar, plus the collapse toggle. */
export function NavUser() {
  const { isMobile, state, toggleSidebar } = useSidebar();
  const { theme, setTheme } = useTheme();
  const { data: versionInfo } = useVersionInfo();
  const version = versionInfo?.current_version;

  const { data: user } = useQuery({
    queryKey: ["current-user"],
    queryFn: getCurrentUser,
    staleTime: 5 * 60 * 1000,
  });

  const displayName = user?.name ?? "User";
  const email = (user as { email?: string } | undefined)?.email;
  const initials = user?.name ? getInitials(user.name) : "?";
  const collapsed = state === "collapsed" && !isMobile;

  const themeItem = (value: "light" | "dark" | "system", label: string, Icon: typeof Sun) => (
    <DropdownMenuItem onClick={() => setTheme(value)} className={cn(theme === value && "bg-accent")}>
      <Icon className="size-4" />
      {label}
    </DropdownMenuItem>
  );

  return (
    <SidebarMenu className={cn("flex-row items-center gap-1", collapsed && "flex-col")}>
      <SidebarMenuItem className="min-w-0 flex-1">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <SidebarMenuButton
              size="lg"
              aria-label="Open user menu"
              className="h-11 gap-2.5 px-1.5 data-[state=open]:bg-sidebar-accent"
            >
              <Avatar className="size-7 shrink-0 rounded-md">
                <AvatarFallback className="rounded-md bg-primary/15 text-xs font-semibold text-primary">
                  {initials}
                </AvatarFallback>
              </Avatar>
              <span className="grid min-w-0 flex-1 text-left leading-tight">
                <span className="truncate text-sm font-medium">{displayName}</span>
                <span className="truncate text-[11px] text-muted-foreground">
                  {email ?? (user?.role ? user.role : "")}
                </span>
              </span>
              <ChevronsUpDown className="ml-auto size-3.5 shrink-0 text-muted-foreground" />
            </SidebarMenuButton>
          </DropdownMenuTrigger>
          <DropdownMenuContent
            className="w-60 rounded-lg"
            side={isMobile ? "top" : "right"}
            align="end"
            sideOffset={6}
          >
            <DropdownMenuLabel className="font-normal">
              <div className="truncate text-sm font-medium">{displayName}</div>
              {email && <div className="truncate text-xs text-muted-foreground">{email}</div>}
            </DropdownMenuLabel>
            <DropdownMenuSeparator />
            <DropdownMenuItem asChild>
              <Link to="/settings/security">
                <UserRound className="size-4" />
                Profile and security
              </Link>
            </DropdownMenuItem>
            <DropdownMenuSub>
              <DropdownMenuSubTrigger>
                <Palette className="size-4" />
                Appearance
              </DropdownMenuSubTrigger>
              <DropdownMenuPortal>
                <DropdownMenuSubContent>
                  {themeItem("light", "Light", Sun)}
                  {themeItem("dark", "Dark", Moon)}
                  {themeItem("system", "System", Monitor)}
                </DropdownMenuSubContent>
              </DropdownMenuPortal>
            </DropdownMenuSub>
            <DropdownMenuSeparator />
            <DropdownMenuItem asChild>
              <a
                href={version ? `${REPO}/releases/tag/${version}` : `${REPO}/releases`}
                target="_blank"
                rel="noopener noreferrer"
              >
                <Sparkles className="size-4" />
                What's new
                {version && <span className="ml-auto font-mono text-[11px] text-muted-foreground">{version}</span>}
              </a>
            </DropdownMenuItem>
            <DropdownMenuItem asChild>
              <a href={`${REPO}#readme`} target="_blank" rel="noopener noreferrer">
                <BookOpen className="size-4" />
                Documentation
              </a>
            </DropdownMenuItem>
            <DropdownMenuItem asChild>
              <a href={`${REPO}/issues/new`} target="_blank" rel="noopener noreferrer">
                <MessageCircle className="size-4" />
                Feedback
              </a>
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem asChild>
              <Link to="/logout">
                <LogOut className="size-4" />
                Log out
              </Link>
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </SidebarMenuItem>
      {!isMobile && (
        <SidebarMenuItem>
          <SidebarMenuButton
            tooltip={collapsed ? "Expand sidebar" : "Collapse sidebar"}
            aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
            onClick={toggleSidebar}
            className="size-8 justify-center p-0 text-muted-foreground"
          >
            {collapsed ? <PanelLeftOpen /> : <PanelLeftClose />}
          </SidebarMenuButton>
        </SidebarMenuItem>
      )}
    </SidebarMenu>
  );
}
