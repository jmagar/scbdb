import type { LocationBrandSummary, LocationsByState } from "../types/api";
import { ErrorState, LoadingState, formatDate } from "./dashboard-utils";
import { StateTileMap } from "./state-tile-map";

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
    data: LocationsByState[] | undefined;
  };
};

export function LocationsPanel({ summary, byState }: Props) {
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

          {/* Interactive US coverage tile map */}
          <h3>US Coverage Map</h3>
          <StateTileMap byState={byState.data} />

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
