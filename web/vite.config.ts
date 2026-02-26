import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";

const DEFAULT_ALLOWED_HOSTS = ["dookie", "scbdb.tootie.tv"];

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "");
  const apiProxyTarget = env.VITE_API_PROXY_TARGET ?? "http://127.0.0.1:3000";
  const allowedHosts = env.VITE_ALLOWED_HOSTS
    ? env.VITE_ALLOWED_HOSTS.split(",")
        .map((h) => h.trim())
        .filter(Boolean)
    : DEFAULT_ALLOWED_HOSTS;

  return {
    plugins: [react()],
    server: {
      port: 5173,
      host: true,
      allowedHosts,
      proxy: {
        "/api": {
          target: apiProxyTarget,
          changeOrigin: true,
        },
      },
    },
    preview: {
      allowedHosts,
      proxy: {
        "/api": {
          target: apiProxyTarget,
          changeOrigin: true,
        },
      },
    },
    test: {
      environment: env.VITEST_TEST_ENV || "node",
      // Tests that exercise HTTP behavior should mock fetch explicitly.
      env: {
        VITE_API_BASE_URL: "",
      },
    },
  };
});
