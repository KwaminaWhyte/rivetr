import { reactRouter } from "@react-router/dev/vite";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [tailwindcss(), reactRouter()],
  resolve: { tsconfigPaths: true },
  server: {
    port: 3000,
    proxy: {
      "/api": {
        target: process.env.API_BASE || "http://localhost:8080",
        changeOrigin: true,
      },
      "/webhooks": {
        target: process.env.API_BASE || "http://localhost:8080",
        changeOrigin: true,
      },
    },
  },
});
