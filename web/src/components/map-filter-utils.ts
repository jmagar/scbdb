import type { LocationBrandSummary } from "../types/api";

export type Relationship = "all" | "portfolio" | "competitor";

/**
 * Extended brand type used by the filter function.
 * `LocationBrandSummary` carries only location stats; `relationship` and `tier`
 * must be joined in from the brands registry before passing here.
 */
export type BrandForFilter = LocationBrandSummary & {
  relationship: "portfolio" | "competitor";
  tier: 1 | 2 | 3;
};

/**
 * Pure function: given the full brand list and filter state, returns the
 * array of brand slugs whose pins should be visible on the map.
 *
 * A slug is visible if ALL three conditions hold:
 *   1. Its brand's `relationship` matches the `relationship` filter (or "all")
 *   2. Its brand's `tier` is in the `tiers` set
 *   3. The slug is in `enabledSlugs` (manual per-brand toggle)
 */
export function computeVisibleSlugs(
  brands: BrandForFilter[],
  relationship: Relationship,
  tiers: Set<number>,
  enabledSlugs: Set<string>,
): string[] {
  return brands
    .filter((b) => {
      if (relationship !== "all" && b.relationship !== relationship)
        return false;
      if (!tiers.has(b.tier)) return false;
      if (!enabledSlugs.has(b.brand_slug)) return false;
      return true;
    })
    .map((b) => b.brand_slug);
}
