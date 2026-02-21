import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "");
  const apiProxyTarget = env.VITE_API_PROXY_TARGET ?? "http://127.0.0.1:3000";

  return {
    plugins: [react()],
    server: {
      port: 5173,
      host: true,
      allowedHosts: ["dookie", "scbdb.tootie.tv"],
      proxy: {
        "/api": {
          target: apiProxyTarget,
          changeOrigin: true,
        },
      },
    },
    preview: {
      allowedHosts: ["dookie", "scbdb.tootie.tv"],
      proxy: {
        "/api": {
          target: apiProxyTarget,
          changeOrigin: true,
        },
      },
    },
    test: {
      // Tests that exercise HTTP behavior should mock fetch explicitly.
      env: {
        VITE_API_BASE_URL: "",
      },
    },
  };
});
