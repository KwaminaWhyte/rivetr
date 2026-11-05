import { useCallback, useEffect, useMemo, useRef, useState } from "react";

/**
 * Form state that knows whether it differs from the saved values.
 *
 * `initial` is the saved state (usually derived from query data). When it changes
 * (compared by value) the form resets to it, so a refetch after saving clears the
 * dirty flag. Pair with <UnsavedChangesBar /> for the sticky save bar.
 */
export interface DirtyForm<T> {
  values: T;
  setValues: React.Dispatch<React.SetStateAction<T>>;
  setField: <K extends keyof T>(key: K, value: T[K]) => void;
  isDirty: boolean;
  /** Fields whose value differs from the saved state. */
  dirtyFields: (keyof T)[];
  /** Discard edits and go back to the saved state. */
  reset: () => void;
  /** Treat the current (or given) values as saved. */
  markSaved: (saved?: T) => void;
}

function stableKey(value: unknown): string {
  return JSON.stringify(value) ?? "";
}

export function useDirtyForm<T extends object>(
  initial: T,
  options: { warnOnLeave?: boolean } = {}
): DirtyForm<T> {
  const { warnOnLeave = true } = options;
  const [values, setValues] = useState<T>(initial);
  const [saved, setSaved] = useState<T>(initial);

  // Re-sync when the saved state from the server changes by value.
  const initialKey = stableKey(initial);
  const lastInitialKey = useRef(initialKey);
  useEffect(() => {
    if (lastInitialKey.current === initialKey) return;
    lastInitialKey.current = initialKey;
    setValues(initial);
    setSaved(initial);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialKey]);

  const dirtyFields = useMemo(
    () =>
      (Object.keys(values) as (keyof T)[]).filter(
        (k) => stableKey(values[k]) !== stableKey(saved[k])
      ),
    [values, saved]
  );
  const isDirty = dirtyFields.length > 0;

  const setField = useCallback(<K extends keyof T>(key: K, value: T[K]) => {
    setValues((prev) => ({ ...prev, [key]: value }));
  }, []);

  const reset = useCallback(() => setValues(saved), [saved]);

  const markSaved = useCallback(
    (next?: T) => {
      const v = next ?? values;
      setSaved(v);
      setValues(v);
    },
    [values]
  );

  // Warn before closing or reloading the tab with unsaved edits.
  useEffect(() => {
    if (!warnOnLeave || !isDirty || typeof window === "undefined") return;
    const handler = (e: BeforeUnloadEvent) => {
      e.preventDefault();
      e.returnValue = "";
    };
    window.addEventListener("beforeunload", handler);
    return () => window.removeEventListener("beforeunload", handler);
  }, [isDirty, warnOnLeave]);

  return { values, setValues, setField, isDirty, dirtyFields, reset, markSaved };
}
