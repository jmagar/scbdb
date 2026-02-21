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
      style={{
        width: isMobileOverlay ? "100%" : "280px",
        flexShrink: 0,
        padding: "1rem",
        overflowY: "auto",
        borderRight: isMobileOverlay
          ? "none"
          : "1px solid var(--border, #e5e7eb)",
      }}
    >
      {isMobileOverlay && (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: "0.75rem",
          }}
        >
          <strong style={{ fontSize: "0.95rem" }}>Filters</strong>
          <button
            type="button"
            aria-label="Close filters"
            onClick={onClose}
            style={{
              background: "none",
              border: "none",
              cursor: "pointer",
              fontSize: "1.25rem",
              lineHeight: 1,
              padding: "0.25rem",
              color: "inherit",
            }}
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
              style={{
                padding: "0.25rem 0.75rem",
                borderRadius: "9999px",
                border: "1px solid var(--border, #e5e7eb)",
                background: relationship === r ? "#1d4ed8" : "transparent",
                color: relationship === r ? "#fff" : "inherit",
                cursor: "pointer",
                fontSize: "0.875rem",
              }}
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

      <hr
        style={{
          margin: "0.75rem 0",
          border: "none",
          borderTop: "1px solid var(--border, #e5e7eb)",
        }}
      />

      {/* Section 2: Tier filter */}
      <section className="filter-section">
        <h4 className="filter-label">Tier</h4>
        <div className="tier-checkboxes" role="group" aria-label="Tier filter">
          {([1, 2, 3] as const).map((tier) => (
            <label
              key={tier}
              style={{
                display: "flex",
                alignItems: "center",
                gap: "0.5rem",
                cursor: "pointer",
                fontSize: "0.875rem",
                marginBottom: "0.25rem",
              }}
            >
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

      <hr
        style={{
          margin: "0.75rem 0",
          border: "none",
          borderTop: "1px solid var(--border, #e5e7eb)",
        }}
      />

      {/* Section 3: Brand list */}
      <section className="filter-section">
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: "0.5rem",
          }}
        >
          <h4 className="filter-label" style={{ margin: 0 }}>
            Brands
          </h4>
          <div style={{ display: "flex", gap: "0.5rem", fontSize: "0.75rem" }}>
            <button
              type="button"
              onClick={selectAllBrands}
              style={{
                background: "none",
                border: "none",
                cursor: "pointer",
                color: "#2563eb",
              }}
            >
              Select all
            </button>
            <button
              type="button"
              onClick={clearAllBrands}
              style={{
                background: "none",
                border: "none",
                cursor: "pointer",
                color: "#6b7280",
              }}
            >
              Clear all
            </button>
          </div>
        </div>
        <div
          className="brand-list"
          style={{ maxHeight: "400px", overflowY: "auto" }}
        >
          {brands.map((brand) => (
            <label
              key={brand.brand_slug}
              style={{
                display: "flex",
                alignItems: "center",
                gap: "0.5rem",
                padding: "0.375rem 0",
                cursor: "pointer",
                fontSize: "0.875rem",
              }}
            >
              <input
                type="checkbox"
                checked={enabledSlugs.has(brand.brand_slug)}
                onChange={() => toggleBrand(brand.brand_slug)}
                aria-label={brand.brand_name}
              />
              {/* Color dot */}
              <span
                aria-hidden="true"
                style={{
                  display: "inline-block",
                  width: "10px",
                  height: "10px",
                  borderRadius: "50%",
                  background: brandColors[brand.brand_slug] ?? "#888",
                  flexShrink: 0,
                }}
              />
              <span style={{ flex: 1 }}>{brand.brand_name}</span>
              <span style={{ color: "#6b7280", fontSize: "0.75rem" }}>
                {brand.active_count.toLocaleString()}
              </span>
            </label>
          ))}
          {brands.length === 0 && (
            <p style={{ color: "#6b7280", fontSize: "0.875rem", margin: 0 }}>
              No brands match current filters.
            </p>
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
