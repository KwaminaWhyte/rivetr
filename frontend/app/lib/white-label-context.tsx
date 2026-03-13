/**
 * White label context.
 * Loads the branding config on app start and makes it available globally.
 * Also applies custom CSS and updates document.title.
 */

import {
  createContext,
  useContext,
  useEffect,
  useRef,
  type ReactNode,
} from "react";
import { useQuery } from "@tanstack/react-query";
import type { WhiteLabel } from "@/lib/api/white-label";

interface WhiteLabelContextValue {
  config: WhiteLabel | null;
  isLoading: boolean;
}

const DEFAULT_CONFIG: WhiteLabel = {
  id: 1,
  app_name: "Rivetr",
  app_description: null,
  logo_url: null,
  favicon_url: null,
  custom_css: null,
  footer_text: null,
  support_url: null,
  docs_url: null,
  login_page_message: null,
  updated_at: "",
};

const WhiteLabelContext = createContext<WhiteLabelContextValue>({
  config: DEFAULT_CONFIG,
  isLoading: false,
});

export function WhiteLabelProvider({ children }: { children: ReactNode }) {
  const styleTagRef = useRef<HTMLStyleElement | null>(null);

  const { data: config, isLoading } = useQuery<WhiteLabel>({
    queryKey: ["white-label"],
    queryFn: async () => {
      const res = await fetch("/api/white-label");
      if (!res.ok) return DEFAULT_CONFIG;
      return res.json();
    },
    staleTime: 1000 * 60 * 5, // 5 minutes
  });

  // Apply document.title
  useEffect(() => {
    if (config?.app_name) {
      // Only update if we haven't already set a page-specific title
      if (document.title === "Rivetr" || document.title === config.app_name) {
        document.title = config.app_name;
      }
    }
  }, [config?.app_name]);

  // Apply custom favicon
  useEffect(() => {
    if (config?.favicon_url) {
      let link = document.querySelector<HTMLLinkElement>("link[rel~='icon']");
      if (!link) {
        link = document.createElement("link");
        link.rel = "icon";
        document.head.appendChild(link);
      }
      link.href = config.favicon_url;
    }
  }, [config?.favicon_url]);

  // Inject / update custom CSS
  useEffect(() => {
    if (config?.custom_css) {
      if (!styleTagRef.current) {
        const tag = document.createElement("style");
        tag.id = "rivetr-white-label-css";
        document.head.appendChild(tag);
        styleTagRef.current = tag;
      }
      styleTagRef.current.textContent = config.custom_css;
    } else if (styleTagRef.current) {
      styleTagRef.current.textContent = "";
    }
  }, [config?.custom_css]);

  return (
    <WhiteLabelContext.Provider
      value={{ config: config ?? DEFAULT_CONFIG, isLoading }}
    >
      {children}
    </WhiteLabelContext.Provider>
  );
}

export function useWhiteLabel(): WhiteLabelContextValue {
  return useContext(WhiteLabelContext);
}
