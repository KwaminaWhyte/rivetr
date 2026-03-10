import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/** Returns the primary domain for an app, checking domains JSON array, legacy domain, then auto_subdomain */
export function getPrimaryDomain(app: {
  domain?: string | null;
  domains?: string | null;
  auto_subdomain?: string | null;
}): string | null {
  if (app.domains) {
    try {
      const parsed = JSON.parse(app.domains) as Array<{ domain: string; primary: boolean }>;
      if (Array.isArray(parsed) && parsed.length > 0) {
        const primary = parsed.find(d => d.primary) || parsed[0];
        return primary.domain;
      }
    } catch {
      // ignore parse errors
    }
  }
  return app.domain || app.auto_subdomain || null;
}
