import { useState } from "react";

import type { LocationsByState } from "../types/api";

// ---------------------------------------------------------------------------
// US State tile grid — standard 11-column geographic layout
// Each entry: [abbreviation, col (0-10), row (0-8)]
// ---------------------------------------------------------------------------
const STATE_TILES: ReadonlyArray<[string, number, number]> = [
  ["ME", 10, 0],
  ["VT", 8, 1],
  ["NH", 9, 1],
  ["WA", 0, 2],
  ["MT", 1, 2],
  ["ND", 2, 2],
  ["MN", 3, 2],
  ["WI", 4, 2],
  ["MI", 7, 2],
  ["NY", 9, 2],
  ["MA", 10, 2],
  ["OR", 0, 3],
  ["ID", 1, 3],
  ["SD", 2, 3],
  ["IA", 3, 3],
  ["IL", 4, 3],
  ["IN", 5, 3],
  ["OH", 6, 3],
  ["PA", 7, 3],
  ["NJ", 8, 3],
  ["CT", 9, 3],
  ["RI", 10, 3],
  ["CA", 0, 4],
  ["NV", 1, 4],
  ["WY", 2, 4],
  ["NE", 3, 4],
  ["MO", 4, 4],
  ["KY", 5, 4],
  ["WV", 6, 4],
  ["VA", 7, 4],
  ["MD", 8, 4],
  ["DE", 9, 4],
  ["AZ", 1, 5],
  ["UT", 2, 5],
  ["CO", 3, 5],
  ["KS", 4, 5],
  ["TN", 5, 5],
  ["NC", 6, 5],
  ["SC", 7, 5],
  ["DC", 8, 5],
  ["NM", 1, 6],
  ["OK", 3, 6],
  ["AR", 4, 6],
  ["MS", 5, 6],
  ["AL", 6, 6],
  ["GA", 7, 6],
  ["TX", 1, 7],
  ["LA", 4, 7],
  ["FL", 6, 7],
  ["AK", 0, 8],
  ["HI", 7, 8],
];

const GRID_COLS = 11;
const GRID_ROWS = 9;

function tileIntensity(count: number): string {
  if (count === 0) return "tile-empty";
  if (count <= 5) return "tile-low";
  if (count <= 25) return "tile-mid";
  if (count <= 100) return "tile-high";
  return "tile-max";
}

type StateTileMapProps = {
  byState: LocationsByState[] | undefined;
};

export function StateTileMap({ byState }: StateTileMapProps) {
  const [tooltip, setTooltip] = useState<{
    state: string;
    brands: number;
    locs: number;
    x: number;
    y: number;
  } | null>(null);

  const stateMap = new Map(
    (byState ?? []).map((s) => [
      s.state,
      { brands: s.brand_count, locs: s.location_count },
    ]),
  );

  // Build full 11×9 grid; empty cells are null
  const grid: (string | null)[][] = Array.from({ length: GRID_ROWS }, () =>
    Array<string | null>(GRID_COLS).fill(null),
  );
  for (const [abbr, col, row] of STATE_TILES) {
    const gridRow = grid[row];
    if (gridRow !== undefined) {
      gridRow[col] = abbr;
    }
  }

  return (
    <div className="tile-map-wrap">
      <div
        className="tile-map"
        style={{
          display: "grid",
          gridTemplateColumns: `repeat(${GRID_COLS}, 1fr)`,
          gap: "3px",
        }}
        onMouseLeave={() => setTooltip(null)}
      >
        {grid.flatMap((row, ri) =>
          row.map((abbr, ci) => {
            if (!abbr) {
              return (
                <div
                  key={`empty-${ri}-${ci}`}
                  className="tile tile-placeholder"
                />
              );
            }
            const data = stateMap.get(abbr);
            const count = data?.locs ?? 0;
            return (
              <button
                key={abbr}
                type="button"
                className={`tile ${tileIntensity(count)}`}
                onMouseEnter={(e) => {
                  const rect = e.currentTarget.getBoundingClientRect();
                  const wrap = e.currentTarget
                    .closest(".tile-map-wrap")
                    ?.getBoundingClientRect();
                  setTooltip({
                    state: abbr,
                    brands: data?.brands ?? 0,
                    locs: count,
                    x: rect.left - (wrap?.left ?? 0) + rect.width / 2,
                    y: rect.top - (wrap?.top ?? 0),
                  });
                }}
              >
                {abbr}
              </button>
            );
          }),
        )}
      </div>

      {tooltip && (
        <div
          className="tile-tooltip"
          style={{ left: tooltip.x, top: tooltip.y }}
        >
          <strong>{tooltip.state}</strong>
          <span>
            {tooltip.locs} location{tooltip.locs !== 1 ? "s" : ""}
          </span>
          <span>
            {tooltip.brands} brand{tooltip.brands !== 1 ? "s" : ""}
          </span>
        </div>
      )}

      <div className="tile-legend">
        <span className="tile-legend-label">Coverage:</span>
        <span className="tile tile-empty tile-legend-item">0</span>
        <span className="tile tile-low tile-legend-item">1–5</span>
        <span className="tile tile-mid tile-legend-item">6–25</span>
        <span className="tile tile-high tile-legend-item">26–100</span>
        <span className="tile tile-max tile-legend-item">100+</span>
      </div>
    </div>
  );
}
