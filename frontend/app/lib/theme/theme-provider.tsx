/**
 * Theme state for the whole dashboard.
 *
 * - Settings live per user on the server (`/api/preferences/theme`) and are
 *   cached in localStorage so the pre-paint script can apply them before the
 *   first frame.
 * - When the server has no endpoint (older backend) or nobody is signed in,
 *   the theme is kept locally only.
 * - Users who never customised get the instance default an admin picked.
 */
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";

import { preferencesApi } from "@/lib/api/preferences";
import { getStoredToken } from "@/lib/api/core";
import { applyTheme, resolveMode } from "./apply";
import { buildThemeOutput } from "./palette";
import { readThemeCache, THEME_CACHE_KEY, writeThemeCache } from "./storage";
import { DEFAULT_THEME, normalizeTheme, themesEqual, type ThemeMode, type ThemeSettings } from "./types";

/** Where the theme is saved right now. */
export type ThemeStorage = "server" | "local";

export interface ThemeContextValue {
  settings: ThemeSettings;
  resolvedMode: "light" | "dark";
  /** False while the user is on a default (instance or built-in). */
  customised: boolean;
  /** Theme an admin set for users who never customised theirs. */
  instanceDefault: ThemeSettings | null;
  storage: ThemeStorage;
  /** Change some fields; saves locally at once and to the server shortly after. */
  update: (patch: Partial<ThemeSettings>) => void;
  /** Replace the whole theme (import). */
  replace: (next: unknown) => void;
  /** Go back to the instance default (or the built-in one) and forget the user's theme. */
  reset: () => void;
  /** Load the signed-in user's theme. Safe to call repeatedly. */
  syncFromServer: () => Promise<void>;
  /** Admin: set or clear (null) the instance default theme. */
  saveInstanceDefault: (theme: ThemeSettings | null) => Promise<void>;

  /** Back-compat with the old mode-only provider. */
  theme: ThemeMode;
  setTheme: (mode: ThemeMode) => void;
  resolvedTheme: "light" | "dark";
}

const ThemeContext = createContext<ThemeContextValue | undefined>(undefined);

const useIsomorphicLayoutEffect = typeof window === "undefined" ? useEffect : useLayoutEffect;

const SAVE_DELAY_MS = 600;

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<ThemeSettings>(DEFAULT_THEME);
  const [customised, setCustomised] = useState(false);
  const [instanceDefault, setInstanceDefault] = useState<ThemeSettings | null>(null);
  const [storage, setStorage] = useState<ThemeStorage>("local");
  const [resolvedMode, setResolvedMode] = useState<"light" | "dark">("light");
  const [ready, setReady] = useState(false);

  // Refs mirror state for async callbacks.
  const settingsRef = useRef(settings);
  const customisedRef = useRef(customised);
  const serverOk = useRef<boolean | null>(null);
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  settingsRef.current = settings;
  customisedRef.current = customised;

  // Load the cached theme before the first paint after hydration.
  useIsomorphicLayoutEffect(() => {
    const cached = readThemeCache();
    if (cached) {
      setSettings(cached.settings);
      setCustomised(cached.customised);
    }
    setReady(true);
  }, []);

  // Apply + cache whenever the theme changes.
  useIsomorphicLayoutEffect(() => {
    if (!ready) return;
    const output = buildThemeOutput(settings);
    applyTheme(settings, output);
    writeThemeCache(settings, customised, output);
    setResolvedMode(resolveMode(settings.mode));
  }, [settings, customised, ready]);

  // Follow the OS while in "system" mode.
  useEffect(() => {
    if (settings.mode !== "system" || typeof window === "undefined" || !window.matchMedia) return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => {
      const mode = mq.matches ? "dark" : "light";
      document.documentElement.classList.toggle("dark", mode === "dark");
      setResolvedMode(mode);
    };
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, [settings.mode]);

  // Another tab changed the theme.
  useEffect(() => {
    const onStorage = (e: StorageEvent) => {
      if (e.key !== THEME_CACHE_KEY) return;
      const cached = readThemeCache();
      if (cached && !themesEqual(cached.settings, settingsRef.current)) {
        setSettings(cached.settings);
        setCustomised(cached.customised);
      }
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  const pushToServer = useCallback((theme: ThemeSettings | null) => {
    if (serverOk.current === false || !getStoredToken()) return;
    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(() => {
      preferencesApi
        .updateThemePreferences(theme)
        .then(() => {
          serverOk.current = true;
          setStorage("server");
        })
        .catch(() => {
          // Older backend without the endpoint, or offline: keep it local.
          serverOk.current = false;
          setStorage("local");
        });
    }, SAVE_DELAY_MS);
  }, []);

  useEffect(
    () => () => {
      if (saveTimer.current) clearTimeout(saveTimer.current);
    },
    []
  );

  const update = useCallback(
    (patch: Partial<ThemeSettings>) => {
      const next = normalizeTheme({ ...settingsRef.current, ...patch });
      setSettings(next);
      setCustomised(true);
      pushToServer(next);
    },
    [pushToServer]
  );

  const replace = useCallback(
    (input: unknown) => {
      const next = normalizeTheme(input);
      setSettings(next);
      setCustomised(true);
      pushToServer(next);
    },
    [pushToServer]
  );

  const reset = useCallback(() => {
    setSettings(instanceDefault ?? DEFAULT_THEME);
    setCustomised(false);
    pushToServer(null);
  }, [instanceDefault, pushToServer]);

  const syncFromServer = useCallback(async () => {
    if (!getStoredToken()) return;
    let res;
    try {
      res = await preferencesApi.getThemePreferences();
    } catch {
      serverOk.current = false;
      setStorage("local");
      return;
    }
    serverOk.current = true;
    setStorage("server");
    const def = res.instance_default ? normalizeTheme(res.instance_default) : null;
    setInstanceDefault(def);
    if (res.theme) {
      setSettings(normalizeTheme(res.theme));
      setCustomised(true);
    } else if (customisedRef.current) {
      // Theme picked on this device before the server knew about it: upload it.
      pushToServer(settingsRef.current);
    } else {
      setSettings(def ?? DEFAULT_THEME);
      setCustomised(false);
    }
  }, [pushToServer]);

  const saveInstanceDefault = useCallback(async (theme: ThemeSettings | null) => {
    const res = await preferencesApi.updateDefaultTheme(theme);
    const def = res.instance_default ? normalizeTheme(res.instance_default) : null;
    setInstanceDefault(def);
    if (!customisedRef.current) setSettings(def ?? DEFAULT_THEME);
  }, []);

  const value = useMemo<ThemeContextValue>(
    () => ({
      settings,
      resolvedMode,
      customised,
      instanceDefault,
      storage,
      update,
      replace,
      reset,
      syncFromServer,
      saveInstanceDefault,
      theme: settings.mode,
      setTheme: (mode: ThemeMode) => update({ mode }),
      resolvedTheme: resolvedMode,
    }),
    [settings, resolvedMode, customised, instanceDefault, storage, update, replace, reset, syncFromServer, saveInstanceDefault]
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeContextValue {
  const context = useContext(ThemeContext);
  if (context === undefined) {
    throw new Error("useTheme must be used within a ThemeProvider");
  }
  return context;
}

/**
 * Load the signed-in user's theme from the server once their id is known.
 * Mounted from the user menu so it runs on every authenticated page.
 */
export function useThemeServerSync(userId: string | undefined) {
  const { syncFromServer } = useTheme();
  useEffect(() => {
    if (userId) void syncFromServer();
  }, [userId, syncFromServer]);
}
