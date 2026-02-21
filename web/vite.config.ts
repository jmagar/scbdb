import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "");
  if (mode !== "test" && !env.VITE_API_BASE_URL) {
    throw new Error(
      `VITE_API_BASE_URL is required for mode "${mode}". Set it in env or web/.env.${mode}.`,
    );
  }

  return {
    plugins: [react()],
    server: {
      port: 5173,
      host: true,
    },
    preview: {
      allowedHosts: ["dookie"],
    },
    test: {
      // Provide a placeholder base URL so client.ts does not throw at import
      // time during vitest runs. Tests that exercise actual HTTP calls must
      // mock fetch themselves.
      env: {
        VITE_API_BASE_URL: "http://localhost:3000",
      },
    },
  };
});
