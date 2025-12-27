"use client";

import * as React from "react";
import { useNavigate } from "react-router";
import { ChevronsUpDown, Plus, Users } from "lucide-react";

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
import type { TeamWithMemberCount } from "@/types/api";

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

  return (
    <SidebarMenu>
      <SidebarMenuItem>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <SidebarMenuButton
              size="lg"
              className="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
            >
              <div className="bg-sidebar-primary text-sidebar-primary-foreground flex aspect-square size-8 items-center justify-center rounded-lg text-xs font-semibold">
                {activeTeam ? (
                  getInitials(activeTeam.name)
                ) : (
                  <Users className="size-4" />
                )}
              </div>
              <div className="grid flex-1 text-left text-sm leading-tight">
                <span className="truncate font-medium">
                  {activeTeam ? activeTeam.name : "Personal"}
                </span>
                <span className="truncate text-xs text-muted-foreground">
                  {activeTeam
                    ? `${activeTeam.member_count} member${activeTeam.member_count !== 1 ? "s" : ""}`
                    : "Your personal workspace"}
                </span>
              </div>
              <ChevronsUpDown className="ml-auto" />
            </SidebarMenuButton>
          </DropdownMenuTrigger>
          <DropdownMenuContent
            className="w-[--radix-dropdown-menu-trigger-width] min-w-56 rounded-lg"
            align="start"
            side={isMobile ? "bottom" : "right"}
            sideOffset={4}
          >
            <DropdownMenuLabel className="text-muted-foreground text-xs">
              Workspaces
            </DropdownMenuLabel>

            {/* Personal Workspace Option */}
            <DropdownMenuItem
              onClick={() => onTeamChange?.(null)}
              className="gap-2 p-2"
            >
              <div className="flex size-6 items-center justify-center rounded-md border bg-muted">
                <Users className="size-3.5 shrink-0" />
              </div>
              <div className="flex-1">
                <span>Personal</span>
                <span className="ml-2 text-xs text-muted-foreground">
                  (default)
                </span>
              </div>
              {!currentTeamId && (
                <span className="text-xs text-primary">Active</span>
              )}
            </DropdownMenuItem>

            {teams.length > 0 && <DropdownMenuSeparator />}

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
                <div className="flex-1 truncate">{team.name}</div>
                {team.id === currentTeamId && (
                  <span className="text-xs text-primary">Active</span>
                )}
              </DropdownMenuItem>
            ))}

            <DropdownMenuSeparator />
            <DropdownMenuItem
              className="gap-2 p-2"
              onClick={() => navigate("/settings/teams")}
            >
              <div className="flex size-6 items-center justify-center rounded-md border bg-transparent">
                <Plus className="size-4" />
              </div>
              <div className="text-muted-foreground font-medium">
                Manage Teams
              </div>
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </SidebarMenuItem>
    </SidebarMenu>
  );
}
