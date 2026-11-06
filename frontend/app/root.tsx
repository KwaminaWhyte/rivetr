import {
  isRouteErrorResponse,
  Link,
  Links,
  Meta,
  Outlet,
  Scripts,
  ScrollRestoration,
} from "react-router";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

import type { Route } from "./+types/root";
import { ThemeProvider, THEME_PREPAINT_SCRIPT } from "@/lib/theme";
import { Toaster } from "@/components/ui/sonner";
import { WhiteLabelProvider } from "@/lib/white-label-context";
import { DeployPanelProvider } from "@/lib/deploy-panel-context";
import { DeploySidePanel } from "@/components/deploy-side-panel";
import { Button } from "@/components/ui/button";
import "./app.css";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60,
      retry: 1,
    },
  },
});

// The theme pre-paint script (see lib/theme/storage.ts) applies the cached
// theme before the first frame so there is no flash of the default look.
const themeScript = THEME_PREPAINT_SCRIPT;

export function Layout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <link id="rivetr-favicon" rel="icon" href="/favicon.ico" />
        <link rel="apple-touch-icon" href="/apple-touch-icon.png" />
        <script dangerouslySetInnerHTML={{ __html: themeScript }} />
        <Meta />
        <Links />
      </head>
      <body>
        <QueryClientProvider client={queryClient}>
          <ThemeProvider>
            <WhiteLabelProvider>
              <DeployPanelProvider>
                {children}
                <DeploySidePanel />
              </DeployPanelProvider>
            </WhiteLabelProvider>
            <Toaster position="top-right" richColors />
          </ThemeProvider>
        </QueryClientProvider>
        <ScrollRestoration />
        <Scripts />
      </body>
    </html>
  );
}

export default function App() {
  return <Outlet />;
}

/**
 * Last-resort page when a route throws. It renders outside the app shell, so it
 * carries its own frame and uses the same tokens as the rest of the dashboard.
 */
export function ErrorBoundary({ error }: Route.ErrorBoundaryProps) {
  const status = isRouteErrorResponse(error) ? error.status : 500;
  const notFound = status === 404;
  const title = notFound ? "Page not found" : "Something went wrong";
  const details = notFound
    ? "This page does not exist or has moved. Use search (Cmd+K) to find what you need."
    : isRouteErrorResponse(error) && error.statusText
      ? error.statusText
      : "The page could not be loaded. Try again, and check the deployment logs if it keeps happening.";
  const stack =
    import.meta.env.DEV && !isRouteErrorResponse(error) && error instanceof Error
      ? error.stack
      : undefined;

  return (
    <main className="flex min-h-svh items-center justify-center bg-background p-6">
      <div className="w-full max-w-md rounded-xl border bg-card p-8 text-center">
        <span className="inline-flex items-center gap-1.5 rounded-full border bg-muted px-2.5 py-1 text-xs font-medium text-muted-foreground">
          <span className="size-1.5 rounded-full bg-destructive" />
          {status}
        </span>
        <h1 className="mt-5 text-xl font-semibold tracking-tight">{title}</h1>
        <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
          {details}
        </p>
        <div className="mt-6 flex items-center justify-center gap-2">
          <Button variant="outline" onClick={() => window.history.back()}>
            Go back
          </Button>
          <Button asChild>
            <Link to="/">Dashboard</Link>
          </Button>
        </div>
        {stack && (
          <pre className="mt-6 max-h-64 overflow-auto rounded-lg bg-muted p-3 text-left text-xs">
            <code>{stack}</code>
          </pre>
        )}
      </div>
    </main>
  );
}
