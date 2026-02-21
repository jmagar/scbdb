import { useMemo, useState } from "react";

import { getBrandColors } from "../lib/brand-colors";
import type { LocationBrandSummary } from "../types/api";
import { useLocationPins } from "../hooks/use-dashboard-data";
import { ErrorState, LoadingState, formatDate } from "./dashboard-utils";
import { LocationMapView } from "./location-map-view";
import { computeVisibleSlugs } from "./map-filter-utils";
import type { BrandForFilter, Relationship } from "./map-filter-utils";
import { MapFilterSidebar } from "./map-filter-sidebar";

function sourceLabel(source: string | null): string {
  switch (source) {
    case "locally":
      return "Locally.com";
    case "storemapper":
      return "Storemapper";
    case "jsonld":
      return "JSON-LD";
    case "json_embed":
      return "Embedded JSON";
    default:
      return source ?? "—";
  }
}

type Props = {
  summary: {
    isLoading: boolean;
    isError: boolean;
    data: LocationBrandSummary[] | undefined;
  };
  byState: {
    isLoading: boolean;
    isError: boolean;
    data:
      | { state: string; brand_count: number; location_count: number }[]
      | undefined;
  };
};

export function LocationsPanel({ summary, byState }: Props) {
  const pins = useLocationPins();

  // Filter state
  const [relationship, setRelationship] = useState<Relationship>("all");
  const [tiers, setTiers] = useState<Set<1 | 2 | 3>>(new Set([1, 2, 3]));
  // null = "all brands enabled" (initial state before user makes any selection)
  const [enabledSlugs, setEnabledSlugs] = useState<Set<string> | null>(null);
  // Mobile: whether the filter overlay is open
  const [filterOpen, setFilterOpen] = useState(false);

  // Derived: brands with relationship/tier, sourced from pins data
  const brandsForFilter = useMemo((): BrandForFilter[] => {
    if (!pins.data || !summary.data) return [];
    // Build a slug → meta map from pin data (pins have relationship + tier)
    const pinMeta = new Map<
      string,
      { relationship: "portfolio" | "competitor"; tier: 1 | 2 | 3 }
    >();
    for (const pin of pins.data) {
      if (!pinMeta.has(pin.brand_slug)) {
        const rel = pin.brand_relationship;
        const tier = pin.brand_tier;
        // Guard against unknown values that could arrive from future DB changes
        if (rel !== "portfolio" && rel !== "competitor") continue;
        if (tier !== 1 && tier !== 2 && tier !== 3) continue;
        pinMeta.set(pin.brand_slug, { relationship: rel, tier });
      }
    }
    return summary.data
      .map((b) => {
        const meta = pinMeta.get(b.brand_slug);
        if (!meta) return null;
        return { ...b, ...meta };
      })
      .filter((b): b is BrandForFilter => b !== null);
  }, [pins.data, summary.data]);

  // null means "all brands enabled"; resolve to a concrete Set for downstream use
  const effectiveEnabledSlugs = useMemo(
    () => enabledSlugs ?? new Set(brandsForFilter.map((b) => b.brand_slug)),
    [enabledSlugs, brandsForFilter],
  );

  // Brand colors
  const brandColors = useMemo(
    () => getBrandColors(brandsForFilter.map((b) => b.brand_slug)),
    [brandsForFilter],
  );

  // Compute visible slugs from filter state
  const selectedSlugs = useMemo(
    () =>
      computeVisibleSlugs(
        brandsForFilter,
        relationship,
        tiers,
        effectiveEnabledSlugs,
      ),
    [brandsForFilter, relationship, tiers, effectiveEnabledSlugs],
  );

  // Top stats
  const totalActive = (summary.data ?? []).reduce(
    (acc, b) => acc + b.active_count,
    0,
  );
  const totalNew = (summary.data ?? []).reduce(
    (acc, b) => acc + b.new_this_week,
    0,
  );
  const statesCovered = byState.data?.length ?? 0;

  return (
    <>
      <h2>Store Coverage</h2>

      {(summary.isLoading || byState.isLoading) && (
        <LoadingState label="store locations" />
      )}
      {(summary.isError || byState.isError) && (
        <ErrorState label="store locations" />
      )}

      {!summary.isLoading && !summary.isError && summary.data && (
        <>
          {/* Top-line stat bar */}
          <div className="locations-stats-bar">
            <div className="locations-stat">
              <strong>{totalActive.toLocaleString()}</strong>
              <span>Active locations</span>
            </div>
            <div className="locations-stat">
              <strong>+{totalNew.toLocaleString()}</strong>
              <span>New this week</span>
            </div>
            <div className="locations-stat">
              <strong>{statesCovered}</strong>
              <span>States covered</span>
            </div>
            <div className="locations-stat">
              <strong>{summary.data.length}</strong>
              <span>Brands tracked</span>
            </div>
          </div>

          {/* Interactive map + filter sidebar */}
          <h3>US Coverage Map</h3>
          <div className="map-layout">
            {/* Desktop sidebar — hidden on mobile via CSS */}
            <div className="map-desktop-sidebar">
              <MapFilterSidebar
                brands={brandsForFilter}
                brandColors={brandColors}
                relationship={relationship}
                setRelationship={setRelationship}
                tiers={tiers}
                setTiers={setTiers}
                enabledSlugs={effectiveEnabledSlugs}
                setEnabledSlugs={setEnabledSlugs}
              />
            </div>

            {/* Map area */}
            <div className="map-canvas-area">
              <LocationMapView
                pins={pins.data ?? []}
                selectedSlugs={selectedSlugs}
                brandColors={brandColors}
                isLoading={pins.isLoading}
                isError={pins.isError}
              />

              {/* Mobile filter toggle button */}
              <button
                type="button"
                className="map-filter-toggle-btn"
                aria-label="Open map filters"
                aria-expanded={filterOpen}
                onClick={() => setFilterOpen(true)}
              >
                <span aria-hidden="true">⚙</span> Filters
              </button>
            </div>

            {/* Mobile overlay — only rendered on small screens via CSS */}
            <MapFilterSidebar
              brands={brandsForFilter}
              brandColors={brandColors}
              relationship={relationship}
              setRelationship={setRelationship}
              tiers={tiers}
              setTiers={setTiers}
              enabledSlugs={effectiveEnabledSlugs}
              setEnabledSlugs={setEnabledSlugs}
              isOpen={filterOpen}
              onClose={() => setFilterOpen(false)}
            />
          </div>

          {/* Per-brand cards */}
          <h3>By Brand</h3>
          <div className="card-stack">
            {summary.data.map((item) => (
              <article className="data-card" key={item.brand_slug}>
                <header>
                  <h3>{item.brand_name}</h3>
                  {item.locator_source && (
                    <span className="source-badge">
                      {sourceLabel(item.locator_source)}
                    </span>
                  )}
                </header>
                <dl>
                  <div>
                    <dt>Active</dt>
                    <dd>{item.active_count.toLocaleString()}</dd>
                  </div>
                  <div>
                    <dt>New (7d)</dt>
                    <dd>+{item.new_this_week}</dd>
                  </div>
                  <div>
                    <dt>States</dt>
                    <dd>{item.states_covered}</dd>
                  </div>
                  <div>
                    <dt>Last seen</dt>
                    <dd>{formatDate(item.last_seen_at)}</dd>
                  </div>
                </dl>
              </article>
            ))}
          </div>

          {/* State breakdown table */}
          {byState.data && byState.data.length > 0 && (
            <>
              <h3>State Breakdown</h3>
              <div
                className="mini-table"
                role="table"
                aria-label="locations-by-state"
              >
                {byState.data.map((item) => (
                  <div className="mini-row" role="row" key={item.state}>
                    <span>{item.state}</span>
                    <strong>{item.location_count.toLocaleString()} loc</strong>
                    <span>
                      {item.brand_count} brand
                      {item.brand_count !== 1 ? "s" : ""}
                    </span>
                  </div>
                ))}
              </div>
            </>
          )}

          {summary.data.length === 0 && (
            <p className="panel-status">
              No location data yet. Run <code>collect locations</code> to
              populate.
            </p>
          )}
        </>
      )}
    </>
  );
}
