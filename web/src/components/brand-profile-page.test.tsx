import { describe, expect, it, vi, beforeEach } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { BrandProfilePage } from "./brand-profile-page";

vi.mock("../hooks/use-dashboard-data", () => ({
  useBrandProfile: vi.fn(),
  useBrandSignals: vi.fn(),
  useBrandFunding: vi.fn(),
  useBrandLabTests: vi.fn(),
  useBrandLegal: vi.fn(),
}));

import {
  useBrandProfile,
  useBrandSignals,
  useBrandFunding,
  useBrandLabTests,
  useBrandLegal,
} from "../hooks/use-dashboard-data";

const mockProfile = {
  id: 1,
  slug: "test-brand",
  name: "Test Brand",
  relationship: "competitor",
  tier: 1,
  logo_url: null,
  profile: {
    tagline: "Test tagline",
    description: null,
    founded_year: 2020,
    hq_city: "Austin",
    hq_state: "TX",
    hq_country: "US",
    parent_company: null,
    ceo_name: null,
    employee_count_approx: null,
    total_funding_usd: null,
    latest_valuation_usd: null,
    funding_stage: null,
  },
  social_handles: [
    {
      platform: "instagram",
      handle: "@testbrand",
      profile_url: "https://instagram.com/testbrand",
      follower_count: 5000,
      is_verified: false,
    },
  ],
  domains: [],
  completeness: {
    score: 42,
    has_profile: true,
    has_description: false,
    has_tagline: true,
    has_founded_year: true,
    has_location: true,
    has_social_handles: true,
    has_domains: false,
    has_signals: false,
    has_funding: false,
    has_lab_tests: false,
    has_legal: false,
    has_sponsorships: false,
    has_distributors: false,
    has_media: false,
  },
};

const emptySignals = {
  data: {
    pages: [{ items: [], next_cursor: null }],
    pageParams: [undefined],
  },
  isLoading: false,
  error: null,
  fetchNextPage: vi.fn(),
  hasNextPage: false,
  isFetchingNextPage: false,
};
const emptyList = { data: [], isLoading: false, error: null };

describe("BrandProfilePage", () => {
  beforeEach(() => {
    vi.mocked(useBrandProfile).mockReturnValue({
      data: mockProfile,
      isLoading: false,
      error: null,
    } as any);
    vi.mocked(useBrandSignals).mockReturnValue(emptySignals as any);
    vi.mocked(useBrandFunding).mockReturnValue(emptyList as any);
    vi.mocked(useBrandLabTests).mockReturnValue(emptyList as any);
    vi.mocked(useBrandLegal).mockReturnValue(emptyList as any);
  });

  it("renders brand name in header", () => {
    const html = renderToStaticMarkup(<BrandProfilePage slug="test-brand" />);
    expect(html).toContain("Test Brand");
  });

  it("renders loading state", () => {
    vi.mocked(useBrandProfile).mockReturnValue({
      data: undefined,
      isLoading: true,
      error: null,
    } as any);
    const html = renderToStaticMarkup(<BrandProfilePage slug="test-brand" />);
    expect(html.toLowerCase()).toContain("loading");
  });

  it("renders error state", () => {
    vi.mocked(useBrandProfile).mockReturnValue({
      data: undefined,
      isLoading: false,
      error: new Error("failed"),
    } as any);
    const html = renderToStaticMarkup(<BrandProfilePage slug="test-brand" />);
    expect(html.toLowerCase()).toContain("failed");
  });

  it("renders tier badge", () => {
    const html = renderToStaticMarkup(<BrandProfilePage slug="test-brand" />);
    expect(html).toContain("T1");
  });

  it("renders relationship badge", () => {
    const html = renderToStaticMarkup(<BrandProfilePage slug="test-brand" />);
    expect(html).toContain("competitor");
  });

  it("renders tagline when present", () => {
    const html = renderToStaticMarkup(<BrandProfilePage slug="test-brand" />);
    expect(html).toContain("Test tagline");
  });

  it("renders back link to brands", () => {
    const html = renderToStaticMarkup(<BrandProfilePage slug="test-brand" />);
    expect(html).toContain("#/brands");
  });

  it("renders completeness bar with correct score", () => {
    const html = renderToStaticMarkup(<BrandProfilePage slug="test-brand" />);
    expect(html).toContain("42%");
  });

  it("renders founding year from profile", () => {
    const html = renderToStaticMarkup(<BrandProfilePage slug="test-brand" />);
    expect(html).toContain("2020");
  });

  it("renders HQ city and state from profile", () => {
    const html = renderToStaticMarkup(<BrandProfilePage slug="test-brand" />);
    expect(html).toContain("Austin");
    expect(html).toContain("TX");
  });

  it("renders social handle platform link", () => {
    const html = renderToStaticMarkup(<BrandProfilePage slug="test-brand" />);
    expect(html).toContain("instagram");
    expect(html).toContain("https://instagram.com/testbrand");
  });

  it("renders Feed, Content, and Recon tabs", () => {
    const html = renderToStaticMarkup(<BrandProfilePage slug="test-brand" />);
    expect(html).toContain("Feed");
    expect(html).toContain("Content");
    expect(html).toContain("Recon");
  });

  it("does not render meta item when parent_company is null", () => {
    const html = renderToStaticMarkup(<BrandProfilePage slug="test-brand" />);
    // parent_company is null in mockProfile, so no parent company label
    expect(html).not.toContain("Parent");
  });
});
