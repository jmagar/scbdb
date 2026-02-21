import { StrictMode, useState, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

import { DashboardPage } from "./components/dashboard-page";
import { BrandsPage } from "./components/brands-page";
import { BrandProfilePage } from "./components/brand-profile-page";
import "./styles.css";

const queryClient = new QueryClient();

function App() {
  const [hash, setHash] = useState(window.location.hash);

  useEffect(() => {
    const handleHashChange = () => setHash(window.location.hash);
    window.addEventListener("hashchange", handleHashChange);
    return () => window.removeEventListener("hashchange", handleHashChange);
  }, []);

  if (hash.startsWith("#/brands/")) {
    const slug = hash.replace("#/brands/", "");
    return <BrandProfilePage slug={slug} />;
  }
  if (hash === "#/brands") {
    return <BrandsPage />;
  }
  return <DashboardPage />;
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
