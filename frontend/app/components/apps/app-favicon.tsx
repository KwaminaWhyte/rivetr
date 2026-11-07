import { useEffect, useState, type ReactNode } from "react";

import { api } from "@/lib/api";
import { cn } from "@/lib/utils";

interface AppFaviconProps {
  appId: string;
  /** Rendered while loading and when the app serves no icon. */
  fallback: ReactNode;
  className?: string;
  /** Empty by default: the app name is always next to the icon. */
  alt?: string;
}

/**
 * The icon an application serves, shown the way Vercel shows a deployment's
 * favicon. The backend fetches and caches it; this component only holds the
 * object URL and falls back to the type icon when there is none.
 */
export function AppFavicon({ appId, fallback, className, alt = "" }: AppFaviconProps) {
  const [url, setUrl] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    setUrl(null);
    void api.getAppFavicon(appId).then((next) => {
      if (alive) setUrl(next);
    });
    return () => {
      alive = false;
    };
  }, [appId]);

  if (!url) return <>{fallback}</>;

  return (
    <img
      src={url}
      alt={alt}
      aria-hidden={alt ? undefined : "true"}
      className={cn("size-4 shrink-0 rounded-sm object-contain", className)}
    />
  );
}
