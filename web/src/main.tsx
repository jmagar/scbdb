import { StrictMode, useState, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

import { DashboardPage } from "./components/dashboard-page";
import { BrandsPage } from "./components/brands-page";
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
    // BrandProfilePage will be added in the next task; show placeholder for now
    return (
      <div style={{ padding: "2rem", fontFamily: "system-ui, sans-serif" }}>
        <a href="#/brands" style={{ color: "#16cba6", textDecoration: "none" }}>
          ← Back to Brands
        </a>
        <h1 style={{ marginTop: "1rem" }}>Brand: {slug}</h1>
        <p style={{ color: "#5f8279" }}>Profile page coming soon.</p>
      </div>
    );
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
