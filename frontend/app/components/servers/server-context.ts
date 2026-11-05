import { useOutletContext } from "react-router";
import type { ServerSummary } from "@/lib/api/server-ops";

/** Outlet context provided by the server layout to its section pages. */
export interface ServerContext {
  serverId: string;
  isLocal: boolean;
  name: string;
  summary: ServerSummary | undefined;
}

export function useServerContext() {
  return useOutletContext<ServerContext>();
}
