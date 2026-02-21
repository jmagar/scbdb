import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import type { LocationPin } from "../types/api";

// Mock maplibre-gl before importing the component
vi.mock("maplibre-gl", () => ({
  default: {
    Map: vi.fn().mockReturnValue({
      on: vi.fn(),
      addControl: vi.fn(),
      remove: vi.fn(),
      addSource: vi.fn(),
      addLayer: vi.fn(),
      setFilter: vi.fn(),
      getSource: vi.fn(),
      isStyleLoaded: vi.fn().mockReturnValue(false),
    }),
    NavigationControl: vi.fn(),
  },
}));

import { LocationMapView } from "./location-map-view";

const samplePins: LocationPin[] = [
  {
    latitude: 30.2672,
    longitude: -97.7431,
    store_name: "Austin Store",
    address_line1: "123 Main St",
    city: "Austin",
    state: "TX",
    zip: "78701",
    locator_source: "locally",
    brand_name: "Cann",
    brand_slug: "cann",
    brand_relationship: "portfolio",
    brand_tier: 1,
  },
];

describe("LocationMapView", () => {
  it("renders the map container div", () => {
    const html = renderToStaticMarkup(
      LocationMapView({
        pins: samplePins,
        selectedSlugs: ["cann"],
        brandColors: { cann: "#e63946" },
        isLoading: false,
        isError: false,
      }),
    );
    // Should render a container element
    expect(html).toBeTruthy();
    expect(typeof html).toBe("string");
  });

  it("renders loading state when isLoading=true", () => {
    const html = renderToStaticMarkup(
      LocationMapView({
        pins: [],
        selectedSlugs: [],
        brandColors: {},
        isLoading: true,
        isError: false,
      }),
    );
    expect(html).toContain("Loading");
  });

  it("renders error state when isError=true", () => {
    const html = renderToStaticMarkup(
      LocationMapView({
        pins: [],
        selectedSlugs: [],
        brandColors: {},
        isLoading: false,
        isError: true,
      }),
    );
    expect(
      html.toLowerCase().includes("error") ||
        html.toLowerCase().includes("failed"),
    ).toBe(true);
  });

  it("renders empty-selection overlay when selectedSlugs is empty", () => {
    const html = renderToStaticMarkup(
      LocationMapView({
        pins: samplePins,
        selectedSlugs: [],
        brandColors: {},
        isLoading: false,
        isError: false,
      }),
    );
    expect(
      html.toLowerCase().includes("brand") ||
        html.toLowerCase().includes("select") ||
        html.toLowerCase().includes("filter"),
    ).toBe(true);
  });
});
