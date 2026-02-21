import { describe, expect, it } from "vitest";

import type {
  BrandProfileResponse,
  BrandSummaryItem,
  SignalItem,
} from "./types/api";

describe("web dashboard", () => {
  it("retains baseline passing test", () => {
    expect(true).toBe(true);
  });
});

describe("brand types", () => {
  it("brand types compile", () => {
    const item: BrandSummaryItem = {
      id: 1,
      slug: "cann",
      name: "Cann",
      tier: 1,
      relationship: "competitor",
      logo_url: null,
      completeness_score: 75,
    };
    expect(item.slug).toBe("cann");
  });

  it("BrandProfileResponse type compiles", () => {
    const profile: BrandProfileResponse = {
      id: 1,
      slug: "cann",
      name: "Cann",
      relationship: "competitor",
      tier: 1,
      domain: null,
      shop_url: null,
      store_locator_url: null,
      twitter_handle: null,
      notes: null,
      logo_url: null,
      profile: null,
      social_handles: [],
      domains: [],
      completeness: {
        score: 0,
        has_profile: false,
        has_description: false,
        has_tagline: false,
        has_founded_year: false,
        has_location: false,
        has_social_handles: false,
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
    expect(profile.slug).toBe("cann");
  });

  it("SignalItem type compiles", () => {
    const signal: SignalItem = {
      id: 1,
      public_id: "abc123",
      signal_type: "article",
      title: "Test Article",
      summary: null,
      source_url: "https://example.com",
      image_url: null,
      published_at: null,
      collected_at: "2026-02-21T00:00:00Z",
    };
    expect(signal.signal_type).toBe("article");
  });
});
