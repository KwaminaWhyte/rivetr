/**
 * Old dashboard URLs and where they live now. routes.ts registers every
 * `from` path against routes/redirects.tsx, which looks the target up here.
 * `:params` carry over by name; the query string and hash are kept.
 *
 * Keep this file free of browser or "@/" imports: routes.ts loads it in Node.
 */

export const LEGACY_REDIRECTS: ReadonlyArray<readonly [from: string, to: string]> = [
  // Applications: settings sub-tabs and duplicates folded into one section each
  ["apps/:id/network", "/apps/:id/domains"],
  ["apps/:id/env-vars", "/apps/:id/environment"],
  ["apps/:id/monitoring", "/apps/:id/metrics"],
  ["apps/:id/settings", "/apps/:id/build"],
  ["apps/:id/settings/build", "/apps/:id/build"],
  ["apps/:id/settings/network", "/apps/:id/domains"],
  ["apps/:id/settings/storage", "/apps/:id/storage"],
  ["apps/:id/settings/healthcheck", "/apps/:id/healthcheck"],
  ["apps/:id/settings/replicas", "/apps/:id/resources"],
  ["apps/:id/settings/docker", "/apps/:id/advanced"],
  ["apps/:id/settings/patches", "/apps/:id/advanced"],
  ["apps/:id/settings/deployment", "/apps/:id/advanced"],
  ["apps/:id/settings/snapshots", "/apps/:id/rollback"],
  ["apps/:id/settings/alerts", "/apps/:id/alerts"],
  ["apps/:id/settings/security", "/apps/:id/security"],
  ["apps/:id/settings/sharing", "/apps/:id/sharing"],

  // Databases and services
  ["databases/:id/settings", "/databases/:id/configuration"],
  ["services/:id/settings", "/services/:id/compose"],
  ["services/:id/stats", "/services/:id"],

  // Projects
  ["projects/:id/env-vars", "/projects/:id/variables"],
  ["projects/:id/apps/new", "/projects/:id/new"],

  // Monitoring
  ["costs", "/monitoring/costs"],
  ["settings/proxy-logs", "/monitoring/proxy-logs"],

  // Servers
  ["build-servers", "/servers/build"],
  ["swarm", "/servers/swarm"],
  ["settings/servers", "/servers"],
  ["settings/build-servers", "/servers/build"],
  ["settings/swarm", "/servers/swarm"],

  // Integrations
  ["git-providers", "/integrations/git"],
  ["settings/git-providers", "/integrations/git"],
  ["settings/s3", "/integrations/s3"],
  ["settings/destinations", "/integrations/destinations"],
  ["tunnels", "/integrations/tunnels"],
  ["settings/tunnels", "/integrations/tunnels"],
  ["webhooks", "/integrations/webhooks"],
  ["settings/webhooks", "/integrations/webhooks"],
  ["webhook-events", "/integrations/webhook-events"],
  ["settings/webhook-events", "/integrations/webhook-events"],

  // Team (teams/:id switches to that team first, see routes/redirects.tsx)
  ["teams/:id", "/team"],
  ["settings/teams", "/teams"],
  ["settings/teams/:id", "/team"],

  // Notifications
  ["settings/notifications", "/notifications"],
  ["settings/alert-defaults", "/notifications/alerts"],

  // Keys and tokens
  ["ssh-keys", "/keys/ssh"],
  ["settings/ssh-keys", "/keys/ssh"],
  ["tokens", "/keys/api-tokens"],
  ["settings/tokens", "/keys/api-tokens"],
  ["settings/ca-certificates", "/keys/ca-certificates"],

  // Account
  ["settings/security", "/account/security"],
  ["settings/preferences", "/account/preferences"],
  ["profile", "/account"],

  // Instance settings
  ["settings/auto-update", "/settings/updates"],
  ["settings/white-label", "/settings/branding"],
  ["settings/oauth", "/settings/authentication"],
  ["settings/sso", "/settings/authentication"],
];

export const LEGACY_REDIRECT_PATHS: string[] = LEGACY_REDIRECTS.map(([from]) => from);

function toRegex(pattern: string): { re: RegExp; names: string[] } {
  const names: string[] = [];
  const body = pattern
    .split("/")
    .map((seg) => {
      if (seg.startsWith(":")) {
        names.push(seg.slice(1));
        return "([^/]+)";
      }
      return seg.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    })
    .join("/");
  return { re: new RegExp(`^/${body}/?$`), names };
}

export interface LegacyRedirectMatch {
  to: string;
  params: Record<string, string>;
}

/** Resolve an old pathname to its new URL (with `search` and `hash` appended). */
export function resolveLegacyRedirect(
  pathname: string,
  search = "",
  hash = ""
): LegacyRedirectMatch | null {
  for (const [from, to] of LEGACY_REDIRECTS) {
    const { re, names } = toRegex(from);
    const m = pathname.match(re);
    if (!m) continue;
    const params: Record<string, string> = {};
    names.forEach((n, i) => (params[n] = m[i + 1]));
    const target = to.replace(/:([A-Za-z]+)/g, (_, n: string) => params[n] ?? "");
    return { to: `${target}${search}${hash}`, params };
  }
  return null;
}
