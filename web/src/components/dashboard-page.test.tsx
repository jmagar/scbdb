import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";

import { DashboardPage } from "./dashboard-page";

vi.mock("maplibre-gl", () => ({
  default: {
    Map: vi.fn().mockReturnValue({
      on: vi.fn(),
      addControl: vi.fn(),
      remove: vi.fn(),
      addSource: vi.fn(),
      addLayer: vi.fn(),
      setFilter: vi.fn(),
      getSource: vi.fn(),
      isStyleLoaded: vi.fn().mockReturnValue(false),
    }),
    NavigationControl: vi.fn(),
  },
}));

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
    useBills: () => ({
      isLoading: false,
      isError: false,
      data: [
        {
          bill_id: "00000000-0000-0000-0000-000000000001",
          jurisdiction: "SC",
          session: "2025-2026",
          bill_number: "HB1234",
          title: "Hemp Beverage Regulation Act",
          status: "introduced",
          status_date: "2026-01-10",
          last_action_date: "2026-01-15",
          source_url: "https://legiscan.com/SC/bill/HB1234/2025",
          event_count: 3,
        },
        {
          bill_id: "00000000-0000-0000-0000-000000000002",
          jurisdiction: "TX",
          session: "2025",
          bill_number: "SB555",
          title: "THC Beverage Prohibition Act",
          status: "failed",
          status_date: "2026-02-01",
          last_action_date: "2026-02-01",
          source_url: null,
          event_count: 1,
        },
      ],
    }),
    useBillEvents: () => ({ isLoading: false, isError: false, data: [] }),
    useBillTexts: () => ({ isLoading: false, isError: false, data: [] }),
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
          metadata: {
            source_counts: {
              google_news: 9,
              reddit_post: 6,
              reddit_comment: 3,
              brand_newsroom: 2,
              twitter_brand: 1,
              twitter_replies: 1,
            },
            top_signals: [
              {
                source: "google_news",
                url: "https://example.com/news/cann-positive",
                score: 0.7,
                text_preview: "Cann sees strong beverage demand this quarter.",
              },
            ],
          },
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
          metadata: {
            source_counts: {
              google_news: 9,
              reddit_post: 6,
              reddit_comment: 3,
              brand_newsroom: 2,
              twitter_brand: 1,
              twitter_replies: 1,
            },
          },
        },
        {
          brand_name: "Cann",
          brand_slug: "cann",
          score: "0.300",
          signal_count: 14,
          captured_at: "2026-02-19T00:00:00Z",
          metadata: {
            source_counts: {
              google_news: 7,
              reddit_post: 5,
              reddit_comment: 2,
              brand_newsroom: 1,
              twitter_brand: 1,
            },
          },
        },
      ],
    }),
    useLocationsSummary: () => ({ isLoading: false, isError: false, data: [] }),
    useLocationsByState: () => ({ isLoading: false, isError: false, data: [] }),
    useLocationPins: () => ({ isLoading: false, isError: false, data: [] }),
  };
});

describe("DashboardPage", () => {
  it("renders core dashboard headings", () => {
    const html = renderToStaticMarkup(<DashboardPage />);
    expect(html).toContain("Southern Crown CBD DB");
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

  it("renders regulatory tab with bill cards and status badges", () => {
    const html = renderToStaticMarkup(
      <DashboardPage initialTab="regulatory" />,
    );
    expect(html).toContain("Regulatory Timeline");
    // Both bills are rendered as buttons
    expect(html).toContain("SC HB1234");
    expect(html).toContain("TX SB555");
    expect(html).toContain("Hemp Beverage Regulation Act");
    expect(html).toContain("THC Beverage Prohibition Act");
    // Status badges with correct colour modifiers
    expect(html).toContain("bill-status-badge"); // base class present
    expect(html).toContain('"bill-status-badge"'); // introduced → no modifier (default grey)
    expect(html).toContain("bill-status-badge--failed"); // TX SB555 failed
    // Chevron affordance present on every card
    const chevronCount = (html.match(/bill-card-chevron/g) ?? []).length;
    expect(chevronCount).toBe(2);
  });

  it("renders sentiment panel with context and transparency", () => {
    const html = renderToStaticMarkup(<DashboardPage initialTab="sentiment" />);
    expect(html).toContain("Market Sentiment");
    expect(html).toContain("Cann");
    expect(html).toContain("+0.42");
    expect(html).toContain("sentiment-badge--positive");
    expect(html).toContain("Data Transparency");
    expect(html).toContain("Google News RSS");
    expect(html).toContain("Reddit");
    expect(html).toContain("brand newsroom posts");
    expect(html).toContain("Twitter/X");
    expect(html).toContain("Momentum");
    expect(html).toContain("+0.12");
    expect(html).toContain("Source mix:");
    expect(html).toContain("Google News (25)");
    expect(html).toContain("Brand Newsroom (5)");
    expect(html).toContain("Twitter Brand Posts (3)");
    expect(html).toContain("Twitter Replies (2)");
    expect(html).toContain("Sample evidence:");
    expect(html).toContain("https://example.com/news/cann-positive");
  });
});
