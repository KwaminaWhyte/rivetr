/**
 * Global "deploy side panel" context.
 *
 * Lets any deploy/start/restart button anywhere in the dashboard open the
 * Coolify-style side panel that streams live logs of an in-progress deploy
 * (apps), service start, or managed-database start. The panel itself lives
 * at the application root so it stays mounted while the user navigates.
 */
import { createContext, useCallback, useContext, useMemo, useState } from "react";
import type { ReactNode } from "react";

export type DeployTargetKind = "deployment" | "service" | "database";

export interface DeployPanelTarget {
  /** Discriminator — what this stream is for. */
  kind: DeployTargetKind;
  /**
   * Resource id:
   * - kind=deployment: deployment id (apps already create deployment rows)
   * - kind=service: service id
   * - kind=database: managed database id
   */
  id: string;
  /** Display name shown in the panel header (e.g. "myapp", "redis-cache"). */
  title: string;
  /** Optional subtitle — usually a short kind label like "App", "Service". */
  subtitle?: string;
  /**
   * For app deployments only — link to the full deployment detail page so the
   * user can jump from the panel into the dedicated logs view.
   */
  href?: string;
}

interface DeployPanelContextValue {
  target: DeployPanelTarget | null;
  open: boolean;
  show: (target: DeployPanelTarget) => void;
  hide: () => void;
  setOpen: (open: boolean) => void;
}

const DeployPanelContext = createContext<DeployPanelContextValue | null>(null);

export function DeployPanelProvider({ children }: { children: ReactNode }) {
  const [target, setTarget] = useState<DeployPanelTarget | null>(null);
  const [open, setOpen] = useState(false);

  const show = useCallback((t: DeployPanelTarget) => {
    setTarget(t);
    setOpen(true);
  }, []);

  const hide = useCallback(() => {
    setOpen(false);
  }, []);

  const value = useMemo<DeployPanelContextValue>(
    () => ({ target, open, show, hide, setOpen }),
    [target, open, show, hide],
  );

  return (
    <DeployPanelContext.Provider value={value}>
      {children}
    </DeployPanelContext.Provider>
  );
}

/** Hook used by deploy/start/restart buttons to open the panel. */
export function useDeployPanel() {
  const ctx = useContext(DeployPanelContext);
  if (!ctx) {
    throw new Error("useDeployPanel must be used within a DeployPanelProvider");
  }
  return ctx;
}
