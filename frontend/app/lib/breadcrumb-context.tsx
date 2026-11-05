import { createContext, useContext, useState, useCallback, useMemo, type ReactNode } from "react";
import { useLocation } from "react-router";

export interface BreadcrumbItem {
  label: string;
  href?: string;
}

interface BreadcrumbContextType {
  items: BreadcrumbItem[];
  setItems: (items: BreadcrumbItem[]) => void;
}

const BreadcrumbContext = createContext<BreadcrumbContextType | null>(null);

/**
 * Items are tagged with the pathname they were set on, so crumbs from the
 * previous page never leak into the next one (child effects run before the
 * layout's, which made a reset-on-navigate effect race with page effects).
 */
export function BreadcrumbProvider({ children }: { children: ReactNode }) {
  const { pathname } = useLocation();
  const [state, setState] = useState<{ path: string; items: BreadcrumbItem[] }>({
    path: "",
    items: [],
  });
  const setItems = useCallback((items: BreadcrumbItem[]) => {
    setState({ path: typeof window === "undefined" ? "" : window.location.pathname, items });
  }, []);
  const value = useMemo(
    () => ({ items: state.path === pathname ? state.items : [], setItems }),
    [state, pathname, setItems]
  );

  return (
    <BreadcrumbContext.Provider value={value}>
      {children}
    </BreadcrumbContext.Provider>
  );
}

export function useBreadcrumb() {
  const context = useContext(BreadcrumbContext);
  if (!context) {
    throw new Error("useBreadcrumb must be used within a BreadcrumbProvider");
  }
  return context;
}

// Hook for child routes to set breadcrumbs
export function useSetBreadcrumbs(items: BreadcrumbItem[]) {
  const { setItems } = useBreadcrumb();

  // Use useEffect to set breadcrumbs when component mounts
  // This is called in the component body, not in useEffect
  // to ensure it runs on every render when items change
  useState(() => {
    setItems(items);
  });
}
