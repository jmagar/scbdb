import { describe, expect, it } from "vitest";
import { getBrandColor, getBrandColors } from "./brand-colors";

describe("getBrandColor", () => {
  it("returns a valid hex color string", () => {
    const color = getBrandColor("some-brand");
    expect(color).toMatch(/^#[0-9a-f]{6}$/i);
  });

  it("is deterministic — same slug always gets same color", () => {
    const color1 = getBrandColor("cann");
    const color2 = getBrandColor("cann");
    expect(color1).toBe(color2);
  });

  it("handles empty string", () => {
    const color = getBrandColor("");
    expect(color).toMatch(/^#[0-9a-f]{6}$/i);
  });

  it("returns distinct colors for different slugs within palette", () => {
    // Test that different slugs can produce different colors
    // (palette has 12 colors; with enough slugs we'll see variety)
    const slugs = [
      "alpha",
      "beta",
      "gamma",
      "delta",
      "epsilon",
      "zeta",
      "eta",
      "theta",
    ];
    const colors = slugs.map(getBrandColor);
    const unique = new Set(colors);
    expect(unique.size).toBeGreaterThan(1);
  });

  it("always returns a color from the PALETTE", () => {
    const PALETTE = [
      "#e63946",
      "#2a9d8f",
      "#e9c46a",
      "#264653",
      "#f4a261",
      "#457b9d",
      "#a8dadc",
      "#6d6875",
      "#b5838d",
      "#81b29a",
      "#f2cc8f",
      "#3d405b",
    ];
    const slugs = [
      "cann",
      "cycling-frog",
      "hi-fi-hops",
      "artet",
      "pabst",
      "wynk",
      "gruvi",
    ];
    for (const slug of slugs) {
      expect(PALETTE).toContain(getBrandColor(slug));
    }
  });
});

describe("getBrandColors", () => {
  it("builds a slug-to-color map for an array of slugs", () => {
    const slugs = ["cann", "cycling-frog"];
    const map = getBrandColors(slugs);
    expect(Object.keys(map)).toHaveLength(2);
    expect(map["cann"]).toMatch(/^#[0-9a-f]{6}$/i);
    expect(map["cycling-frog"]).toMatch(/^#[0-9a-f]{6}$/i);
  });

  it("handles an empty array", () => {
    const map = getBrandColors([]);
    expect(map).toEqual({});
  });

  it("is consistent with getBrandColor", () => {
    const slugs = ["cann", "artet"];
    const map = getBrandColors(slugs);
    for (const slug of slugs) {
      expect(map[slug]).toBe(getBrandColor(slug));
    }
  });
});
