import type { Config } from "@react-router/dev/config";

export default {
  // Disable SSR - frontend will be served as static files by the Rust backend
  ssr: false,
  // Pre-render the root route to generate index.html for SPA mode
  async prerender() {
    return ["/"];
  },
} satisfies Config;
