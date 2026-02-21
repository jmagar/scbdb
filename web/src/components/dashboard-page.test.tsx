import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";

import { DashboardPage } from "./dashboard-page";

vi.mock("../hooks/use-dashboard-data", () => {
  return {
    useProducts: () => ({ isLoading: false, isError: false, data: [] }),
    usePricingSummary: () => ({
      isLoading: false,
      isError: false,
      data: [
        {
          brand_name: "Brand With Logo",
          brand_slug: "brand-with-logo",
          brand_logo_url: "https://cdn.example.com/brand-logo.svg",
          variant_count: 3,
          avg_price: "24.99",
          min_price: "19.99",
          max_price: "29.99",
          latest_capture_at: "2026-02-21T00:00:00Z",
        },
        {
          brand_name: "Brand Without Logo",
          brand_slug: "brand-without-logo",
          brand_logo_url: null,
          variant_count: 1,
          avg_price: "9.99",
          min_price: "9.99",
          max_price: "9.99",
          latest_capture_at: "2026-02-21T00:00:00Z",
        },
      ],
    }),
    usePricingSnapshots: () => ({
      isLoading: false,
      isError: false,
      data: [
        {
          captured_at: "2026-02-21T00:00:00Z",
          currency_code: "USD",
          price: "24.99",
          compare_at_price: null,
          variant_title: "Default",
          source_variant_id: "v1",
          product_name: "Product A",
          brand_name: "Brand With Logo",
          brand_slug: "brand-with-logo",
          brand_logo_url: "https://cdn.example.com/brand-logo.svg",
        },
        {
          captured_at: "2026-02-21T00:00:00Z",
          currency_code: "USD",
          price: "9.99",
          compare_at_price: null,
          variant_title: "Default",
          source_variant_id: "v2",
          product_name: "Product B",
          brand_name: "Brand Without Logo",
          brand_slug: "brand-without-logo",
          brand_logo_url: null,
        },
      ],
    }),
    useBills: () => ({ isLoading: false, isError: false, data: [] }),
    useSentimentSummary: () => ({
      isLoading: false,
      isError: false,
      data: [
        {
          brand_name: "Cann",
          brand_slug: "cann",
          score: "0.420",
          signal_count: 18,
          captured_at: "2026-02-20T00:00:00Z",
        },
      ],
    }),
    useSentimentSnapshots: () => ({
      isLoading: false,
      isError: false,
      data: [
        {
          brand_name: "Cann",
          brand_slug: "cann",
          score: "0.420",
          signal_count: 18,
          captured_at: "2026-02-20T00:00:00Z",
        },
      ],
    }),
    useLocationsSummary: () => ({ isLoading: false, isError: false, data: [] }),
    useLocationsByState: () => ({ isLoading: false, isError: false, data: [] }),
  };
});

describe("DashboardPage", () => {
  it("renders core dashboard headings", () => {
    const html = renderToStaticMarkup(<DashboardPage />);
    expect(html).toContain("API Dashboard");
    expect(html).toContain("Product Catalog");
  });

  it("renders pricing tab brand logos and fallback placeholders", () => {
    const html = renderToStaticMarkup(<DashboardPage initialTab="pricing" />);
    expect(html).toContain("Pricing Summary");
    expect(html).toContain('src="https://cdn.example.com/brand-logo.svg"');
    expect(html).toContain('alt="Brand With Logo brand"');
    expect(html).toContain("brand-image brand-image-empty");
    expect(html).toContain("mini-brand-fallback");
  });

  it("renders five stat cards", () => {
    const html = renderToStaticMarkup(<DashboardPage />);
    const buttonCount = (html.match(/<button/g) ?? []).length;
    expect(buttonCount).toBe(5);
  });

  it("renders sentiment panel with score badge", () => {
    const html = renderToStaticMarkup(<DashboardPage initialTab="sentiment" />);
    expect(html).toContain("Market Sentiment");
    expect(html).toContain("Cann");
    expect(html).toContain("+0.42");
    expect(html).toContain("sentiment-badge--positive");
  });
});
