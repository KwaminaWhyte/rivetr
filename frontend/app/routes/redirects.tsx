import { redirect } from "react-router";
import { resolveLegacyRedirect } from "@/lib/redirects";

const TEAM_STORAGE_KEY = "rivetr_current_team";

/**
 * Serves every old dashboard URL listed in lib/redirects.ts and sends the
 * browser to the page's new home, keeping ids, the query string and the hash.
 */
export async function clientLoader({ request }: { request: Request }) {
  const url = new URL(request.url);
  const match = resolveLegacyRedirect(url.pathname, url.search, url.hash);
  if (!match) return redirect("/");

  // /teams/:id used to open any team; the team pages now follow the current
  // team, so switch to it before landing on /team.
  if (match.params.id && /^\/(settings\/)?teams\//.test(url.pathname)) {
    try {
      localStorage.setItem(TEAM_STORAGE_KEY, match.params.id);
    } catch {
      // Storage blocked: land on the current team instead.
    }
  }
  return redirect(match.to);
}

export function HydrateFallback() {
  return null;
}

export default function LegacyRedirect() {
  return null;
}
