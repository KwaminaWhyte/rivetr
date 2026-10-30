"use client";

import * as React from "react";
import { useNavigate } from "react-router";
import { ChevronsUpDown, Plus } from "lucide-react";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from "@/components/ui/sidebar";
import { Badge } from "@/components/ui/badge";
import { RivetrLogo } from "@/components/rivetr-logo";
import { useWhiteLabel } from "@/lib/white-label-context";
import type { TeamWithMemberCount, TeamRole } from "@/types/api";

interface TeamSwitcherProps {
  teams: TeamWithMemberCount[];
  currentTeamId?: string | null;
  onTeamChange?: (teamId: string | null) => void;
}

export function TeamSwitcher({
  teams,
  currentTeamId,
  onTeamChange,
}: TeamSwitcherProps) {
  const { isMobile } = useSidebar();
  const { config } = useWhiteLabel();
  const navigate = useNavigate();

  const activeTeam = teams.find((t) => t.id === currentTeamId);

  // Get initials for a team name
  const getInitials = (name: string) => {
    return name
      .split(" ")
      .map((word) => word[0])
      .join("")
      .toUpperCase()
      .slice(0, 2);
  };

  // Get role badge variant based on role
  const getRoleBadgeVariant = (role: TeamRole | null): "default" | "secondary" | "outline" => {
    switch (role) {
      case "owner":
        return "default";
      case "admin":
        return "secondary";
      default:
        return "outline";
    }
  };

  // Check if user can create teams (any user can create teams)
  const canCreateTeam = true;

  return (
    <SidebarMenu>
      <SidebarMenuItem>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <SidebarMenuButton
              size="lg"
              tooltip={activeTeam ? activeTeam.name : "Personal"}
              className="border border-sidebar-border bg-background shadow-xs group-data-[collapsible=icon]:border-0 group-data-[collapsible=icon]:shadow-none data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
            >
              <RivetrLogo decorative className="size-8 rounded-md" />
              <div className="grid flex-1 text-left leading-tight">
                <span className="truncate text-sm font-semibold tracking-tight">
                  {config?.app_name || "Rivetr"}
                </span>
                <span className="truncate text-xs text-muted-foreground">
                  {activeTeam ? activeTeam.name : "Personal"}
                </span>
              </div>
              <ChevronsUpDown className="ml-auto size-4 text-muted-foreground" />
            </SidebarMenuButton>
          </DropdownMenuTrigger>
          <DropdownMenuContent
            className="w-[--radix-dropdown-menu-trigger-width] min-w-56 rounded-lg"
            align="start"
            side={isMobile ? "bottom" : "right"}
            sideOffset={4}
          >
            <DropdownMenuLabel className="text-muted-foreground text-xs">
              Teams
            </DropdownMenuLabel>

            {teams.map((team) => (
              <DropdownMenuItem
                key={team.id}
                onClick={() => onTeamChange?.(team.id)}
                className="gap-2 p-2"
              >
                <div className="flex size-6 items-center justify-center rounded-md border bg-muted text-xs font-semibold">
                  {getInitials(team.name)}
                </div>
                <div className="flex flex-1 items-center gap-2 truncate">
                  <span className="truncate">{team.name}</span>
                  {team.user_role && (
                    <Badge
                      variant={getRoleBadgeVariant(team.user_role)}
                      className="h-5 px-1.5 text-[10px] capitalize"
                    >
                      {team.user_role}
                    </Badge>
                  )}
                </div>
                {team.id === currentTeamId && (
                  <span className="text-xs text-primary">Active</span>
                )}
              </DropdownMenuItem>
            ))}

            <DropdownMenuSeparator />
            {canCreateTeam && (
              <DropdownMenuItem
                className="gap-2 p-2"
                onClick={() => navigate("/teams?create=true")}
              >
                <div className="flex size-6 items-center justify-center rounded-md border bg-transparent">
                  <Plus className="size-4" />
                </div>
                <div className="text-muted-foreground font-medium">
                  Create Team
                </div>
              </DropdownMenuItem>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
      </SidebarMenuItem>
    </SidebarMenu>
  );
}
