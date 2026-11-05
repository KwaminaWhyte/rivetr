/**
 * Local cache of the theme, read by the pre-paint script in root.tsx so the
 * first frame already has the right colours. It holds the computed CSS too,
 * so that script needs no theme logic of its own.
 *
 * Lives outside the `rivetr:ui:*` namespace so "Reset UI layout" keeps it.
 */
import { buildThemeOutput, type ThemeOutput } from "./palette";
import { DEFAULT_THEME, normalizeTheme, THEME_MODES, type ThemeSettings } from "./types";

export const THEME_CACHE_KEY = "rivetr:theme";
/** Pre-theme-system key that only held the mode. */
const LEGACY_MODE_KEY = "rivetr_theme";

export interface ThemeCache extends ThemeOutput {
  settings: ThemeSettings;
  /** True once the user changed something themselves (vs. a default). */
  customised: boolean;
}

export function readThemeCache(): { settings: ThemeSettings; customised: boolean } | null {
  if (typeof window === "undefined") return null;
  try {
    const raw = window.localStorage.getItem(THEME_CACHE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<ThemeCache>;
      return { settings: normalizeTheme(parsed.settings), customised: parsed.customised === true };
    }
    const legacy = window.localStorage.getItem(LEGACY_MODE_KEY);
    if (legacy && (THEME_MODES as readonly string[]).includes(legacy)) {
      return {
        settings: normalizeTheme({ ...DEFAULT_THEME, mode: legacy }),
        customised: legacy !== DEFAULT_THEME.mode,
      };
    }
  } catch {
    // Unreadable cache: fall through to the default.
  }
  return null;
}

export function writeThemeCache(settings: ThemeSettings, customised: boolean, output?: ThemeOutput) {
  if (typeof window === "undefined") return;
  try {
    const cache: ThemeCache = { settings, customised, ...(output ?? buildThemeOutput(settings)) };
    window.localStorage.setItem(THEME_CACHE_KEY, JSON.stringify(cache));
    window.localStorage.removeItem(LEGACY_MODE_KEY);
  } catch {
    // Storage full or blocked: the theme still applies for this page view.
  }
}

/**
 * Runs inline in <head> before the app CSS paints. Kept tiny and defensive:
 * any failure leaves the stock look from app.css.
 */
export const THEME_PREPAINT_SCRIPT = `(function(){try{var d=document.documentElement,ls=window.localStorage,c=null;try{c=JSON.parse(ls.getItem('${THEME_CACHE_KEY}')||'null')}catch(e){}var m=c&&c.settings&&c.settings.mode;if(!m){m=ls.getItem('${LEGACY_MODE_KEY}')||'system'}var dk=m==='dark'||(m!=='light'&&window.matchMedia('(prefers-color-scheme: dark)').matches);if(dk)d.classList.add('dark');if(c&&c.attrs){for(var k in c.attrs){if(/^data-[a-z-]+$/.test(k))d.setAttribute(k,String(c.attrs[k]))}}if(c&&typeof c.css==='string'&&c.css.indexOf('<')<0){var s=document.createElement('style');s.id='rivetr-theme-vars';s.textContent=c.css;document.head.appendChild(s)}}catch(e){}})();`;
