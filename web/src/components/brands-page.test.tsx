import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { BrandsPage } from "./brands-page";

vi.mock("../hooks/use-dashboard-data", () => ({
  useBrands: () => ({
    data: [
      {
        id: 1,
        slug: "cann",
        name: "Cann",
        tier: 1,
        relationship: "competitor",
        logo_url: null,
        completeness_score: 45,
      },
    ],
    isLoading: false,
    error: null,
  }),
}));

describe("BrandsPage", () => {
  it("renders brands heading", () => {
    const html = renderToStaticMarkup(<BrandsPage />);
    expect(html).toContain("<h1");
    expect(html.toLowerCase()).toContain("brands");
  });

  it("renders brand name from data", () => {
    const html = renderToStaticMarkup(<BrandsPage />);
    expect(html).toContain("Cann");
  });

  it("renders brand card link to brand profile", () => {
    const html = renderToStaticMarkup(<BrandsPage />);
    expect(html).toContain("#/brands/cann");
  });

  it("renders back link to dashboard", () => {
    const html = renderToStaticMarkup(<BrandsPage />);
    expect(html).toContain("#/");
  });

  it("renders tier badge", () => {
    const html = renderToStaticMarkup(<BrandsPage />);
    expect(html).toContain("T1");
  });

  it("renders relationship badge", () => {
    const html = renderToStaticMarkup(<BrandsPage />);
    expect(html).toContain("competitor");
  });

  it("renders completeness bar with correct width", () => {
    const html = renderToStaticMarkup(<BrandsPage />);
    expect(html).toContain("45%");
  });
});
