import { describe, expect, it } from "vitest";
import { computeVisibleSlugs } from "./map-filter-utils";
import type { LocationBrandSummary } from "../types/api";

// Use a local extended type for testing
type BrandWithMeta = LocationBrandSummary & {
  relationship: "portfolio" | "competitor";
  tier: 1 | 2 | 3;
};

function makeTestBrand(
  slug: string,
  relationship: "portfolio" | "competitor",
  tier: 1 | 2 | 3,
): BrandWithMeta {
  return {
    brand_name: slug,
    brand_slug: slug,
    active_count: 10,
    new_this_week: 1,
    states_covered: 3,
    locator_source: null,
    last_seen_at: null,
    relationship,
    tier,
  };
}

const brands: BrandWithMeta[] = [
  makeTestBrand("cann", "portfolio", 1),
  makeTestBrand("cycling-frog", "portfolio", 2),
  makeTestBrand("hi-fi-hops", "competitor", 1),
  makeTestBrand("artet", "competitor", 3),
];

describe("computeVisibleSlugs", () => {
  it("returns all slugs when relationship=all and all tiers selected and all enabled", () => {
    const result = computeVisibleSlugs(
      brands,
      "all",
      new Set([1, 2, 3]),
      new Set(brands.map((b) => b.brand_slug)),
    );
    expect(result).toHaveLength(4);
    expect(result).toContain("cann");
    expect(result).toContain("cycling-frog");
    expect(result).toContain("hi-fi-hops");
    expect(result).toContain("artet");
  });

  it("filters to portfolio brands when relationship=portfolio", () => {
    const result = computeVisibleSlugs(
      brands,
      "portfolio",
      new Set([1, 2, 3]),
      new Set(brands.map((b) => b.brand_slug)),
    );
    expect(result).toHaveLength(2);
    expect(result).toContain("cann");
    expect(result).toContain("cycling-frog");
    expect(result).not.toContain("hi-fi-hops");
    expect(result).not.toContain("artet");
  });

  it("filters to competitor brands when relationship=competitor", () => {
    const result = computeVisibleSlugs(
      brands,
      "competitor",
      new Set([1, 2, 3]),
      new Set(brands.map((b) => b.brand_slug)),
    );
    expect(result).toHaveLength(2);
    expect(result).toContain("hi-fi-hops");
    expect(result).toContain("artet");
  });

  it("filters by tier 1 only", () => {
    const result = computeVisibleSlugs(
      brands,
      "all",
      new Set([1]),
      new Set(brands.map((b) => b.brand_slug)),
    );
    expect(result).toHaveLength(2);
    expect(result).toContain("cann");
    expect(result).toContain("hi-fi-hops");
  });

  it("filters by multiple tiers", () => {
    const result = computeVisibleSlugs(
      brands,
      "all",
      new Set([1, 2]),
      new Set(brands.map((b) => b.brand_slug)),
    );
    expect(result).toHaveLength(3);
    expect(result).toContain("cann");
    expect(result).toContain("cycling-frog");
    expect(result).toContain("hi-fi-hops");
    expect(result).not.toContain("artet");
  });

  it("combines relationship + tier filters", () => {
    const result = computeVisibleSlugs(
      brands,
      "portfolio",
      new Set([1]),
      new Set(brands.map((b) => b.brand_slug)),
    );
    expect(result).toHaveLength(1);
    expect(result).toContain("cann");
  });

  it("returns only manually-enabled brands after filter narrowing", () => {
    // all filters pass "cann" and "cycling-frog" (portfolio), but only "cann" is enabled
    const result = computeVisibleSlugs(
      brands,
      "portfolio",
      new Set([1, 2, 3]),
      new Set(["cann"]),
    );
    expect(result).toEqual(["cann"]);
  });

  it("returns empty array when no brands enabled", () => {
    const result = computeVisibleSlugs(
      brands,
      "all",
      new Set([1, 2, 3]),
      new Set([]),
    );
    expect(result).toHaveLength(0);
  });
});
