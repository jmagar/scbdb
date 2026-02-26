import { StrictMode, useState, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

import { DashboardPage } from "./components/dashboard-page";
import { BrandsPage } from "./components/brands-page";
import { BrandProfilePage } from "./components/brand-profile-page";
import { NotFoundPage } from "./components/not-found-page";
import "./styles.css";

export const ROUTES = {
  dashboard: "#/",
  brands: "#/brands",
  brand: (slug: string) => `#/brands/${encodeURIComponent(slug)}`,
} as const;

const SLUG_PATTERN = /^[a-z0-9-]+$/;

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 60_000,
      gcTime: 10 * 60_000,
      refetchOnWindowFocus: false,
      retry: 1,
    },
  },
});

function App() {
  const [hash, setHash] = useState(window.location.hash);

  useEffect(() => {
    const handleHashChange = () => setHash(window.location.hash);
    window.addEventListener("hashchange", handleHashChange);
    return () => window.removeEventListener("hashchange", handleHashChange);
  }, []);

  let decoded = hash;
  try {
    decoded = decodeURIComponent(hash);
  } catch {
    return <NotFoundPage />;
  }

  if (decoded.startsWith("#/brands/")) {
    const slug = decoded.replace("#/brands/", "");
    if (!slug || !SLUG_PATTERN.test(slug)) return <NotFoundPage />;
    return <BrandProfilePage slug={slug} />;
  }
  if (decoded === "#/brands") {
    return <BrandsPage />;
  }
  if (decoded === "" || decoded === "#" || decoded === "#/") {
    return <DashboardPage />;
  }
  return <NotFoundPage />;
}

const rootElement = document.getElementById("root");
if (!rootElement) {
  throw new Error("Root element not found");
}

createRoot(rootElement).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </StrictMode>,
);
