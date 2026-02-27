import type { LocationBrandSummary } from "../types/api";
import type { CSSProperties } from "react";

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

export const MAP_FULL_SIZE_STYLE: CSSProperties = {
  width: "100%",
  height: "100%",
};

export const MAP_WRAPPER_STYLE: CSSProperties = {
  position: "relative",
  width: "100%",
  height: "100%",
};

export const MAP_EMPTY_OVERLAY_STYLE: CSSProperties = {
  position: "absolute",
  inset: 0,
  zIndex: 10,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  background: "rgba(255,255,255,0.7)",
};

export const FILTER_DIVIDER_STYLE: CSSProperties = {
  margin: "0.75rem 0",
  border: "none",
  borderTop: "1px solid var(--border, #e5e7eb)",
};

export const FILTER_SECTION_HEADER_STYLE: CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  marginBottom: "0.5rem",
};

export const FILTER_ACTION_ROW_STYLE: CSSProperties = {
  display: "flex",
  gap: "0.5rem",
  fontSize: "0.75rem",
};

const TEXT_ACTION_BUTTON_BASE_STYLE: CSSProperties = {
  background: "none",
  border: "none",
  cursor: "pointer",
};

export function getTextActionButtonStyle(color: string): CSSProperties {
  return { ...TEXT_ACTION_BUTTON_BASE_STYLE, color };
}

export function getSidebarPanelStyle(isMobileOverlay: boolean): CSSProperties {
  return {
    width: isMobileOverlay ? "100%" : "280px",
    flexShrink: 0,
    padding: "1rem",
    overflowY: "auto",
    borderRight: isMobileOverlay ? "none" : "1px solid var(--border, #e5e7eb)",
  };
}

export const FILTER_MOBILE_HEADER_STYLE: CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  marginBottom: "0.75rem",
};

export const FILTER_CLOSE_BUTTON_STYLE: CSSProperties = {
  background: "none",
  border: "none",
  cursor: "pointer",
  fontSize: "1.25rem",
  lineHeight: 1,
  padding: "0.25rem",
  color: "inherit",
};

export function getPillButtonStyle(isActive: boolean): CSSProperties {
  return {
    padding: "0.25rem 0.75rem",
    borderRadius: "9999px",
    border: "1px solid var(--border, #e5e7eb)",
    background: isActive ? "#1d4ed8" : "transparent",
    color: isActive ? "#fff" : "inherit",
    cursor: "pointer",
    fontSize: "0.875rem",
  };
}

export const FILTER_TIER_ROW_STYLE: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.5rem",
  cursor: "pointer",
  fontSize: "0.875rem",
  marginBottom: "0.25rem",
};

export const FILTER_BRAND_LIST_STYLE: CSSProperties = {
  maxHeight: "400px",
  overflowY: "auto",
};

export const FILTER_BRAND_ROW_STYLE: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.5rem",
  padding: "0.375rem 0",
  cursor: "pointer",
  fontSize: "0.875rem",
};

export const FILTER_BRAND_NAME_STYLE: CSSProperties = {
  flex: 1,
};

export const FILTER_COUNT_STYLE: CSSProperties = {
  color: "#6b7280",
  fontSize: "0.75rem",
};

export const FILTER_EMPTY_STYLE: CSSProperties = {
  color: "#6b7280",
  fontSize: "0.875rem",
  margin: 0,
};

export function getColorDotStyle(color: string): CSSProperties {
  return {
    display: "inline-block",
    width: "10px",
    height: "10px",
    borderRadius: "50%",
    background: color,
    flexShrink: 0,
  };
}

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
