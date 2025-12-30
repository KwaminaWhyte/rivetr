import { createContext, useContext, useState, useCallback, type ReactNode } from "react";

export interface BreadcrumbItem {
  label: string;
  href?: string;
}

interface BreadcrumbContextType {
  items: BreadcrumbItem[];
  setItems: (items: BreadcrumbItem[]) => void;
}

const BreadcrumbContext = createContext<BreadcrumbContextType | null>(null);

export function BreadcrumbProvider({ children }: { children: ReactNode }) {
  const [items, setItems] = useState<BreadcrumbItem[]>([]);

  return (
    <BreadcrumbContext.Provider value={{ items, setItems }}>
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
