import "maplibre-gl/dist/maplibre-gl.css";
import maplibregl from "maplibre-gl";
import { useEffect, useRef } from "react";
import {
  MAP_EMPTY_OVERLAY_STYLE,
  MAP_FULL_SIZE_STYLE,
  MAP_WRAPPER_STYLE,
} from "./map-filter-utils";
import type { LocationPin } from "../types/api";

type Props = {
  pins: LocationPin[];
  selectedSlugs: string[];
  brandColors: Record<string, string>;
  isLoading: boolean;
  isError: boolean;
};

type MapCanvasProps = {
  pins: LocationPin[];
  selectedSlugs: string[];
  brandColors: Record<string, string>;
};

// Build a GeoJSON FeatureCollection from pins + brand colors.
// Extracted so both the init effect and the data-update effect can share it.
// Typed as any: @types/geojson is not installed separately;
// maplibre-gl bundles GeoJSON types internally via @maplibre/geojson-vt.
function buildGeojson(
  pins: LocationPin[],
  brandColors: Record<string, string>,
): any {
  return {
    type: "FeatureCollection",
    features: pins.map((pin) => ({
      type: "Feature",
      geometry: { type: "Point", coordinates: [pin.longitude, pin.latitude] },
      properties: {
        store_name: pin.store_name,
        address_line1: pin.address_line1 ?? "",
        city: pin.city ?? "",
        state: pin.state ?? "",
        zip: pin.zip ?? "",
        locator_source: pin.locator_source ?? "",
        brand_name: pin.brand_name,
        brand_slug: pin.brand_slug,
        color: brandColors[pin.brand_slug] ?? "#888888",
      },
    })),
  };
}

// Inner component: owns all MapLibre hook-based lifecycle.
// Isolated here so the outer LocationMapView can be called as a plain
// function in renderToStaticMarkup tests without violating hook rules.
function MapCanvas({ pins, selectedSlugs, brandColors }: MapCanvasProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<maplibregl.Map | null>(null);

  // Initialize map on mount
  useEffect(() => {
    if (!containerRef.current) return;

    const map = new maplibregl.Map({
      container: containerRef.current,
      style: "https://tiles.openfreemap.org/styles/liberty",
      center: [-96, 39],
      zoom: 4,
    });

    map.addControl(new maplibregl.NavigationControl(), "top-right");
    mapRef.current = map;

    map.on("load", () => {
      const geojson = buildGeojson(pins, brandColors);

      map.addSource("store-pins", {
        type: "geojson",
        data: geojson,
        cluster: true,
        clusterMaxZoom: 14,
        clusterRadius: 50,
      });

      // Cluster circles
      map.addLayer({
        id: "clusters",
        type: "circle",
        source: "store-pins",
        filter: ["has", "point_count"],
        paint: {
          "circle-color": "#374151",
          "circle-radius": [
            "step",
            ["get", "point_count"],
            16,
            10,
            22,
            100,
            30,
          ],
          "circle-opacity": 0.85,
        },
      });

      // Cluster count labels
      map.addLayer({
        id: "cluster-count",
        type: "symbol",
        source: "store-pins",
        filter: ["has", "point_count"],
        layout: {
          "text-field": "{point_count_abbreviated}",
          "text-font": ["Noto Sans Regular"],
          "text-size": 12,
        },
        paint: { "text-color": "#ffffff" },
      });

      // Individual store pins — circle color driven by brand_slug property
      map.addLayer({
        id: "pins",
        type: "circle",
        source: "store-pins",
        filter: ["!", ["has", "point_count"]],
        paint: {
          "circle-color": ["get", "color"],
          "circle-radius": 7,
          "circle-stroke-width": 1.5,
          "circle-stroke-color": "#ffffff",
        },
      });
    });

    return () => {
      map.remove();
      mapRef.current = null;
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Update GeoJSON source data when pins or colors change after initial mount.
  // Without this, the map stays permanently stale after TanStack Query refetches.
  // Also attaches a one-time `load` listener so updates that arrive before
  // the map finishes loading are not silently dropped.
  useEffect(() => {
    const map = mapRef.current;
    if (!map) return;

    const applyData = () => {
      const source = map.getSource("store-pins") as
        | maplibregl.GeoJSONSource
        | undefined;
      if (!source) return;
      source.setData(buildGeojson(pins, brandColors));
    };

    if (map.isStyleLoaded()) {
      applyData();
    } else {
      map.once("load", applyData);
      return () => {
        map.off("load", applyData);
      };
    }
  }, [pins, brandColors]);

  // Update layer filter when selectedSlugs changes.
  // FilterSpecification is not re-exported at maplibre-gl top level in v5;
  // it lives in @maplibre/maplibre-gl-style-spec, so use any here.
  useEffect(() => {
    const map = mapRef.current;
    if (!map) return;

    // When no slugs are selected, apply a filter that matches nothing so no
    // pins are visible (the overlay hides the map visually, but keeping the
    // layer empty is the correct underlying state).
    const slugFilter: any =
      selectedSlugs.length > 0
        ? ["in", ["get", "brand_slug"], ["literal", selectedSlugs]]
        : ["==", ["get", "brand_slug"], ""];

    try {
      map.setFilter("pins", ["all", ["!", ["has", "point_count"]], slugFilter]);
    } catch {
      // Style may not be loaded yet; re-fires when selectedSlugs changes.
    }
  }, [selectedSlugs]);

  return <div ref={containerRef} style={MAP_FULL_SIZE_STYLE} />;
}

// Outer component: pure state-machine — no hooks of its own.
// The early-return paths (isLoading, isError) return plain JSX with no hooks,
// which means renderToStaticMarkup(LocationMapView({...})) works in tests.
// The happy path renders <MapCanvas/> as JSX; renderToStaticMarkup handles it
// as a deferred component, not a direct function call, so hooks in MapCanvas
// are never invoked during SSR (useEffect is a no-op server-side).
export function LocationMapView({
  pins,
  selectedSlugs,
  brandColors,
  isLoading,
  isError,
}: Props) {
  if (isLoading) {
    return (
      <div className="map-loading-state">
        <span>Loading map data&#8230;</span>
      </div>
    );
  }

  if (isError) {
    return (
      <div className="map-error-state">
        <span>Error loading map data. Please try again.</span>
      </div>
    );
  }

  return (
    <div className="location-map-wrapper" style={MAP_WRAPPER_STYLE}>
      {selectedSlugs.length === 0 && (
        <div className="map-empty-overlay" style={MAP_EMPTY_OVERLAY_STYLE}>
          <p>Select at least one brand to filter the map.</p>
        </div>
      )}
      <MapCanvas
        pins={pins}
        selectedSlugs={selectedSlugs}
        brandColors={brandColors}
      />
    </div>
  );
}
