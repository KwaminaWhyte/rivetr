import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useEffect,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";

/**
 * Lets a page put its status pill and primary actions into the shell's top
 * bar (next to the breadcrumbs), Coolify style. On narrow screens the content
 * renders inline where the page placed it instead, so nothing is lost.
 */

type Slot = "status" | "actions";

interface SlotContext {
  targets: Record<Slot, HTMLElement | null>;
  register: (slot: Slot, el: HTMLElement | null) => void;
}

const Ctx = createContext<SlotContext | null>(null);

export function TopBarSlotProvider({ children }: { children: ReactNode }) {
  const [targets, setTargets] = useState<Record<Slot, HTMLElement | null>>({
    status: null,
    actions: null,
  });
  const register = useCallback(
    (slot: Slot, el: HTMLElement | null) =>
      setTargets((prev) => (prev[slot] === el ? prev : { ...prev, [slot]: el })),
    []
  );
  const value = useMemo(() => ({ targets, register }), [targets, register]);
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

/** Placeholder the shell renders where page content for `slot` should appear. */
export function TopBarTarget({ slot, className }: { slot: Slot; className?: string }) {
  const register = useContext(Ctx)?.register;
  const ref = useCallback((el: HTMLDivElement | null) => register?.(slot, el), [register, slot]);
  return <div ref={ref} className={className} />;
}

function useMinWidth(px: number): boolean {
  const [matches, setMatches] = useState(false);
  useEffect(() => {
    const mql = window.matchMedia(`(min-width: ${px}px)`);
    const update = () => setMatches(mql.matches);
    update();
    mql.addEventListener("change", update);
    return () => mql.removeEventListener("change", update);
  }, [px]);
  return matches;
}

/**
 * Render children into the top bar on large screens (>= 1024px), inline
 * otherwise. `inline` wraps the inline fallback (e.g. a flex row).
 */
export function TopBarPortal({
  slot,
  children,
  inlineClassName,
}: {
  slot: Slot;
  children: ReactNode;
  inlineClassName?: string;
}) {
  const ctx = useContext(Ctx);
  const wide = useMinWidth(1024);
  const target = ctx?.targets[slot] ?? null;
  if (wide && target) return createPortal(children, target);
  return <div className={inlineClassName}>{children}</div>;
}
