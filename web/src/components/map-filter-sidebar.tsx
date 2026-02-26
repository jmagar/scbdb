import {
  FILTER_ACTION_ROW_STYLE,
  FILTER_BRAND_LIST_STYLE,
  FILTER_BRAND_NAME_STYLE,
  FILTER_BRAND_ROW_STYLE,
  FILTER_CLOSE_BUTTON_STYLE,
  FILTER_COUNT_STYLE,
  FILTER_DIVIDER_STYLE,
  FILTER_EMPTY_STYLE,
  FILTER_MOBILE_HEADER_STYLE,
  FILTER_SECTION_HEADER_STYLE,
  FILTER_TIER_ROW_STYLE,
  getColorDotStyle,
  getPillButtonStyle,
  getSidebarPanelStyle,
  getTextActionButtonStyle,
} from "./map-filter-utils";
import type { BrandForFilter, Relationship } from "./map-filter-utils";

type MapFilterSidebarProps = {
  brands: BrandForFilter[];
  brandColors: Record<string, string>;
  relationship: Relationship;
  setRelationship: (r: Relationship) => void;
  tiers: Set<1 | 2 | 3>;
  setTiers: (t: Set<1 | 2 | 3>) => void;
  enabledSlugs: Set<string>;
  setEnabledSlugs: (s: Set<string>) => void;
  /** Mobile overlay: whether the panel is open */
  isOpen?: boolean;
  /** Mobile overlay: called when the user closes the panel */
  onClose?: () => void;
};

export function MapFilterSidebar({
  brands,
  brandColors,
  relationship,
  setRelationship,
  tiers,
  setTiers,
  enabledSlugs,
  setEnabledSlugs,
  isOpen,
  onClose,
}: MapFilterSidebarProps) {
  // Tier toggle helper
  function toggleTier(tier: 1 | 2 | 3) {
    const next = new Set(tiers);
    if (next.has(tier)) {
      next.delete(tier);
    } else {
      next.add(tier);
    }
    setTiers(next);
  }

  // Brand enable/disable helper
  function toggleBrand(slug: string) {
    const next = new Set(enabledSlugs);
    if (next.has(slug)) {
      next.delete(slug);
    } else {
      next.add(slug);
    }
    setEnabledSlugs(next);
  }

  function selectAllBrands() {
    setEnabledSlugs(new Set(brands.map((b) => b.brand_slug)));
  }

  function clearAllBrands() {
    setEnabledSlugs(new Set());
  }

  // In mobile-overlay mode, the sidebar is wrapped in a backdrop + slide-up panel.
  // isOpen === undefined means desktop mode (always visible, no overlay behaviour).
  const isMobileOverlay = isOpen !== undefined;

  const panel = (
    <div
      className="map-filter-sidebar"
      style={getSidebarPanelStyle(isMobileOverlay)}
    >
      {isMobileOverlay && (
        <div style={FILTER_MOBILE_HEADER_STYLE}>
          <strong style={{ fontSize: "0.95rem" }}>Filters</strong>
          <button
            type="button"
            aria-label="Close filters"
            onClick={onClose}
            style={FILTER_CLOSE_BUTTON_STYLE}
          >
            ✕
          </button>
        </div>
      )}
      {/* Section 1: Relationship filter */}
      <section className="filter-section">
        <h4 className="filter-label">Relationship</h4>
        <div
          className="pill-toggle"
          role="group"
          aria-label="Relationship filter"
        >
          {(["all", "portfolio", "competitor"] as const).map((r) => (
            <button
              key={r}
              type="button"
              className={`pill-btn ${relationship === r ? "active" : ""}`}
              aria-pressed={relationship === r}
              onClick={() => setRelationship(r)}
              style={getPillButtonStyle(relationship === r)}
            >
              {r === "all"
                ? "All"
                : r === "portfolio"
                  ? "Portfolio"
                  : "Competitors"}
            </button>
          ))}
        </div>
      </section>

      <hr style={FILTER_DIVIDER_STYLE} />

      {/* Section 2: Tier filter */}
      <section className="filter-section">
        <h4 className="filter-label">Tier</h4>
        <div className="tier-checkboxes" role="group" aria-label="Tier filter">
          {([1, 2, 3] as const).map((tier) => (
            <label key={tier} style={FILTER_TIER_ROW_STYLE}>
              <input
                type="checkbox"
                checked={tiers.has(tier)}
                onChange={() => toggleTier(tier)}
                aria-label={`Tier ${tier}`}
              />
              Tier {tier}
            </label>
          ))}
        </div>
      </section>

      <hr style={FILTER_DIVIDER_STYLE} />

      {/* Section 3: Brand list */}
      <section className="filter-section">
        <div style={FILTER_SECTION_HEADER_STYLE}>
          <h4 className="filter-label" style={{ margin: 0 }}>
            Brands
          </h4>
          <div style={FILTER_ACTION_ROW_STYLE}>
            <button
              type="button"
              onClick={selectAllBrands}
              style={getTextActionButtonStyle("#2563eb")}
            >
              Select all
            </button>
            <button
              type="button"
              onClick={clearAllBrands}
              style={getTextActionButtonStyle("#6b7280")}
            >
              Clear all
            </button>
          </div>
        </div>
        <div className="brand-list" style={FILTER_BRAND_LIST_STYLE}>
          {brands.map((brand) => (
            <label key={brand.brand_slug} style={FILTER_BRAND_ROW_STYLE}>
              <input
                type="checkbox"
                checked={enabledSlugs.has(brand.brand_slug)}
                onChange={() => toggleBrand(brand.brand_slug)}
                aria-label={brand.brand_name}
              />
              {/* Color dot */}
              <span
                aria-hidden="true"
                style={getColorDotStyle(
                  brandColors[brand.brand_slug] ?? "#888",
                )}
              />
              <span style={FILTER_BRAND_NAME_STYLE}>{brand.brand_name}</span>
              <span style={FILTER_COUNT_STYLE}>
                {brand.active_count.toLocaleString()}
              </span>
            </label>
          ))}
          {brands.length === 0 && (
            <p style={FILTER_EMPTY_STYLE}>No brands match current filters.</p>
          )}
        </div>
      </section>
    </div>
  );

  if (isMobileOverlay) {
    return (
      <>
        {/* Backdrop — click to close */}
        {isOpen && (
          <div
            className="map-filter-backdrop"
            aria-hidden="true"
            onClick={onClose}
          />
        )}
        {/* Slide-up panel */}
        <div
          className={`map-filter-overlay${isOpen ? " open" : ""}`}
          role="dialog"
          aria-modal="true"
          aria-label="Map filters"
        >
          {panel}
        </div>
      </>
    );
  }

  return panel;
}
