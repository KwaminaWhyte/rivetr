/**
 * Per-browser UI state that survives reloads: view toggles, collapsed panels,
 * table page sizes, log viewer options and so on.
 *
 * - Keys are namespaced `rivetr:ui:<key>` so "Reset UI layout" can clear them.
 * - Values are stored as `{ v: version, d: data }`; a version bump or a value
 *   that fails `validate` falls back to the default.
 * - Every storage access is wrapped in try/catch (private mode, blocked site
 *   data, quota) and the hook renders the default on the server and during
 *   hydration, then the stored value.
 * - Tabs stay in sync through the `storage` event; hooks in the same tab
 *   sharing a key update together.
 */
import { useCallback, useMemo, useRef, useSyncExternalStore } from "react";

export const UI_STATE_PREFIX = "rivetr:ui:";

export type PersistedStorage = "local" | "session";

export interface PersistedStateOptions<T> {
  /** "local" (default) survives restarts; "session" is per tab. */
  storage?: PersistedStorage;
  /** Bump to discard values saved in an older shape. */
  version?: number;
  /** Reject stored values that are not a valid T. */
  validate?: (value: unknown) => value is T;
}

type Listener = () => void;

/** Marks a cache entry that only lives in memory because storage failed. */
const MEMORY = "\u0000memory";

const listeners = new Map<string, Set<Listener>>();
/** Parsed-value cache keyed by full key, so snapshots are referentially stable. */
const cache = new Map<string, { raw: string | null; value: unknown }>();

function getArea(storage: PersistedStorage): Storage | null {
  if (typeof window === "undefined") return null;
  try {
    return storage === "session" ? window.sessionStorage : window.localStorage;
  } catch {
    return null;
  }
}

function readRaw(storage: PersistedStorage, fullKey: string): string | null {
  try {
    return getArea(storage)?.getItem(fullKey) ?? null;
  } catch {
    return null;
  }
}

function emit(fullKey: string) {
  listeners.get(fullKey)?.forEach((l) => l());
}

function subscribeKey(fullKey: string, listener: Listener) {
  let set = listeners.get(fullKey);
  if (!set) {
    set = new Set();
    listeners.set(fullKey, set);
  }
  set.add(listener);
  return () => {
    set.delete(listener);
    if (set.size === 0) listeners.delete(fullKey);
  };
}

let storageListenerInstalled = false;
function installStorageListener() {
  if (storageListenerInstalled || typeof window === "undefined") return;
  storageListenerInstalled = true;
  window.addEventListener("storage", (e) => {
    if (e.key === null) {
      // storage.clear() in another tab
      cache.clear();
      listeners.forEach((set) => set.forEach((l) => l()));
    } else if (e.key.startsWith(UI_STATE_PREFIX)) {
      cache.delete(e.key);
      emit(e.key);
    }
  });
}

/** Snapshot for "nothing valid stored": the hook substitutes the current default. */
const MISSING: unique symbol = Symbol("missing");
type Snapshot<T> = T | typeof MISSING;

function decode<T>(raw: string | null, version: number, validate?: (v: unknown) => v is T): Snapshot<T> {
  if (raw === null) return MISSING;
  try {
    const parsed = JSON.parse(raw) as { v?: unknown; d?: unknown };
    if (!parsed || typeof parsed !== "object" || parsed.v !== version || !("d" in parsed)) return MISSING;
    if (validate && !validate(parsed.d)) return MISSING;
    return parsed.d as T;
  } catch {
    return MISSING;
  }
}

/** Read a persisted value outside React (e.g. in an event handler). */
export function readPersisted<T>(key: string, fallback: T, options: PersistedStateOptions<T> = {}): T {
  const { storage = "local", version = 1, validate } = options;
  const value = decode(readRaw(storage, UI_STATE_PREFIX + key), version, validate);
  return value === MISSING ? fallback : value;
}

/** Write a persisted value outside React. Hooks using the key re-render. */
export function writePersisted<T>(key: string, value: T, options: Pick<PersistedStateOptions<T>, "storage" | "version"> = {}) {
  const { storage = "local", version = 1 } = options;
  const fullKey = UI_STATE_PREFIX + key;
  let saved = false;
  try {
    const area = getArea(storage);
    if (area) {
      area.setItem(fullKey, JSON.stringify({ v: version, d: value }));
      saved = true;
    }
  } catch {
    // Storage unavailable or full: fall through to the in-memory value.
  }
  if (saved) cache.delete(fullKey);
  else cache.set(fullKey, { raw: MEMORY, value });
  emit(fullKey);
}

/** Remove every `rivetr:ui:*` key from local and session storage. Returns how many were removed. */
export function clearPersistedUiState(): number {
  let removed = 0;
  for (const storage of ["local", "session"] as const) {
    const area = getArea(storage);
    if (!area) continue;
    try {
      const keys: string[] = [];
      for (let i = 0; i < area.length; i++) {
        const k = area.key(i);
        if (k && k.startsWith(UI_STATE_PREFIX)) keys.push(k);
      }
      keys.forEach((k) => area.removeItem(k));
      removed += keys.length;
    } catch {
      // ignore
    }
  }
  cache.clear();
  listeners.forEach((set) => set.forEach((l) => l()));
  return removed;
}

/**
 * Like useState, but the value is kept in localStorage (or sessionStorage)
 * under `rivetr:ui:<key>`.
 */
export function usePersistedState<T>(
  key: string,
  defaultValue: T,
  options: PersistedStateOptions<T> = {}
): [T, (next: T | ((prev: T) => T)) => void, () => void] {
  const { storage = "local", version = 1, validate } = options;
  const fullKey = UI_STATE_PREFIX + key;

  const subscribe = useCallback(
    (listener: Listener) => {
      installStorageListener();
      return subscribeKey(fullKey, listener);
    },
    [fullKey]
  );

  const getSnapshot = useCallback((): Snapshot<T> => {
    const memo = cache.get(fullKey);
    if (memo?.raw === MEMORY) return memo.value as T;
    const raw = readRaw(storage, fullKey);
    if (memo && memo.raw === raw) return memo.value as Snapshot<T>;
    const value = decode(raw, version, validate);
    cache.set(fullKey, { raw, value });
    return value;
    // validate is read when the stored text changes; callers pass stable guards.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [fullKey, storage, version]);

  const getServerSnapshot = useCallback((): Snapshot<T> => MISSING, []);

  const snapshot = useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);
  const value = snapshot === MISSING ? defaultValue : snapshot;

  const defaultRef = useRef(defaultValue);
  defaultRef.current = defaultValue;

  const setValue = useCallback(
    (next: T | ((prev: T) => T)) => {
      const current = getSnapshot();
      const prev = current === MISSING ? defaultRef.current : current;
      const resolved = typeof next === "function" ? (next as (p: T) => T)(prev) : next;
      if (current !== MISSING && Object.is(resolved, prev)) return;
      writePersisted(key, resolved, { storage, version });
    },
    [getSnapshot, key, storage, version]
  );

  const reset = useCallback(() => {
    try {
      getArea(storage)?.removeItem(fullKey);
    } catch {
      // ignore
    }
    cache.delete(fullKey);
    emit(fullKey);
  }, [fullKey, storage]);

  return [value, setValue, reset];
}

/**
 * Convenience for a string choice from a fixed list (view modes, tabs, ranges).
 * Anything not in `allowed` reads as the default.
 */
export function useLocalPreference<V extends string>(
  key: string,
  defaultValue: V,
  allowed: readonly V[]
): [V, (value: V) => void] {
  const validate = useMemo(
    () => (v: unknown): v is V => typeof v === "string" && (allowed as readonly string[]).includes(v),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [allowed.join("|")]
  );
  const [value, setValue] = usePersistedState<V>(key, defaultValue, { validate });
  return [value, setValue as (value: V) => void];
}

const EMPTY_LIST: string[] = [];
const MAX_SET_SIZE = 200;

/**
 * A persisted Set<string> (expanded rows, open cards). The setter accepts a
 * Set or an updater like useState, so it can replace `useState<Set<string>>`.
 */
export function usePersistedSet(
  key: string,
  options: Pick<PersistedStateOptions<string[]>, "storage" | "version"> = {}
): [Set<string>, (next: Set<string> | ((prev: Set<string>) => Set<string>)) => void] {
  const [list, setList] = usePersistedState<string[]>(key, EMPTY_LIST, { ...options, validate: isStringArray });
  const set = useMemo(() => new Set(list), [list]);
  const update = useCallback(
    (next: Set<string> | ((prev: Set<string>) => Set<string>)) => {
      setList((prevList) => {
        const resolved = typeof next === "function" ? next(new Set(prevList)) : next;
        return [...resolved].slice(-MAX_SET_SIZE);
      });
    },
    [setList]
  );
  return [set, update];
}

/** Type guards for common shapes, handy as `validate`. */
export const isBoolean = (v: unknown): v is boolean => typeof v === "boolean";
export const isNumber = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
export const isString = (v: unknown): v is string => typeof v === "string";
export const isStringArray = (v: unknown): v is string[] =>
  Array.isArray(v) && v.every((x) => typeof x === "string");
export const oneOf =
  <V extends string | number>(allowed: readonly V[]) =>
  (v: unknown): v is V =>
    (allowed as readonly unknown[]).includes(v);
