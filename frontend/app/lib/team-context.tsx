/**
 * Team Context Provider
 * Manages the current team context across the application.
 * - Stores current team ID in React context and localStorage
 * - Fetches teams list on mount
 * - Provides team switching functionality
 */

import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  type ReactNode,
} from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { teamsApi } from "@/lib/api/teams";
import type { TeamWithMemberCount, TeamRole } from "@/types/api";

const TEAM_STORAGE_KEY = "rivetr_current_team";

interface TeamContextType {
  /** Current team ID (null for personal workspace) */
  currentTeamId: string | null;
  /** Current team object (null for personal workspace) */
  currentTeam: TeamWithMemberCount | null;
  /** All teams the user has access to */
  teams: TeamWithMemberCount[];
  /** Whether teams are loading */
  isLoading: boolean;
  /** Error if teams failed to load */
  error: Error | null;
  /** Switch to a different team */
  setCurrentTeamId: (teamId: string | null) => void;
  /** Current user's role in the current team */
  currentRole: TeamRole | null;
  /** Refresh teams list */
  refetchTeams: () => Promise<void>;
}

const TeamContext = createContext<TeamContextType | null>(null);

/**
 * Get the stored team ID from localStorage
 */
function getStoredTeamId(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem(TEAM_STORAGE_KEY);
}

/**
 * Store the team ID in localStorage
 */
function storeTeamId(teamId: string | null): void {
  if (typeof window === "undefined") return;
  if (teamId) {
    localStorage.setItem(TEAM_STORAGE_KEY, teamId);
  } else {
    localStorage.removeItem(TEAM_STORAGE_KEY);
  }
}

interface TeamProviderProps {
  children: ReactNode;
}

export function TeamProvider({ children }: TeamProviderProps) {
  const queryClient = useQueryClient();
  const [currentTeamId, setCurrentTeamIdState] = useState<string | null>(() =>
    getStoredTeamId()
  );

  // Fetch teams list
  const {
    data: teams = [],
    isLoading,
    error,
    refetch,
  } = useQuery<TeamWithMemberCount[], Error>({
    queryKey: ["teams"],
    queryFn: () => teamsApi.getTeams(),
    staleTime: 1000 * 60 * 5, // 5 minutes
    retry: 1,
  });

  // On mount, validate stored team ID
  useEffect(() => {
    if (!isLoading && teams.length > 0) {
      const storedId = getStoredTeamId();
      if (storedId) {
        // Verify stored team still exists and user has access
        const teamExists = teams.some((t) => t.id === storedId);
        if (!teamExists) {
          // Team no longer accessible, use first team or null
          const firstTeam = teams[0];
          setCurrentTeamIdState(firstTeam?.id ?? null);
          storeTeamId(firstTeam?.id ?? null);
        } else {
          setCurrentTeamIdState(storedId);
        }
      } else if (teams.length > 0) {
        // No stored team, use first team
        setCurrentTeamIdState(teams[0].id);
        storeTeamId(teams[0].id);
      }
    }
  }, [teams, isLoading]);

  // Get current team object
  const currentTeam = teams.find((t) => t.id === currentTeamId) ?? null;

  // Get user's role in current team (would need to be fetched from team details)
  // For now, we'll use null - this will be enhanced when we have user context
  const currentRole: TeamRole | null = null;

  // Switch team handler
  const setCurrentTeamId = useCallback(
    (teamId: string | null) => {
      setCurrentTeamIdState(teamId);
      storeTeamId(teamId);
      // Invalidate queries that depend on team context
      queryClient.invalidateQueries({ queryKey: ["apps"] });
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      queryClient.invalidateQueries({ queryKey: ["databases"] });
      queryClient.invalidateQueries({ queryKey: ["services"] });
      queryClient.invalidateQueries({ queryKey: ["stats"] });
    },
    [queryClient]
  );

  // Refetch teams
  const refetchTeams = useCallback(async () => {
    await refetch();
  }, [refetch]);

  const value: TeamContextType = {
    currentTeamId,
    currentTeam,
    teams,
    isLoading,
    error: error ?? null,
    setCurrentTeamId,
    currentRole,
    refetchTeams,
  };

  return <TeamContext.Provider value={value}>{children}</TeamContext.Provider>;
}

/**
 * Hook to access team context
 * @throws Error if used outside TeamProvider
 */
export function useTeamContext() {
  const context = useContext(TeamContext);
  if (!context) {
    throw new Error("useTeamContext must be used within a TeamProvider");
  }
  return context;
}

/**
 * Hook to get the current team ID for API calls
 * Returns undefined for personal workspace (no team filtering)
 */
export function useCurrentTeamId(): string | undefined {
  const { currentTeamId } = useTeamContext();
  return currentTeamId ?? undefined;
}

/**
 * Hook to check if user is in a team context
 */
export function useIsTeamContext(): boolean {
  const { currentTeamId } = useTeamContext();
  return currentTeamId !== null;
}
