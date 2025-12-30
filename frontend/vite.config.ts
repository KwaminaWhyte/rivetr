import { reactRouter } from "@react-router/dev/vite";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import tsconfigPaths from "vite-tsconfig-paths";

export default defineConfig({
  plugins: [tailwindcss(), reactRouter(), tsconfigPaths()],
  server: {
    port: 3000,
    proxy: {
      "/api": {
        target: process.env.API_BASE || "http://localhost:9080",
        changeOrigin: true,
      },
      "/webhooks": {
        target: process.env.API_BASE || "http://localhost:9080",
        changeOrigin: true,
      },
    },
  },
});
