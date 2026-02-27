import { describe, expect, it } from "vitest";

import {
  formatDate,
  formatMoney,
  formatScore,
  scoreClass,
  scorePct,
  trimText,
} from "./dashboard-utils";

// ---------------------------------------------------------------------------
// formatMoney
// ---------------------------------------------------------------------------
describe("formatMoney", () => {
  it("formats a typical price string", () => {
    expect(formatMoney("24.99")).toBe("$24.99");
  });

  it("formats zero", () => {
    expect(formatMoney("0")).toBe("$0.00");
  });

  it("rounds to 2 decimal places", () => {
    expect(formatMoney("9.999")).toBe("$10.00");
  });

  it("passes through non-numeric strings unchanged", () => {
    expect(formatMoney("n/a")).toBe("n/a");
  });

  it("treats an empty string as zero (Number('') === 0)", () => {
    expect(formatMoney("")).toBe("$0.00");
  });
});

// ---------------------------------------------------------------------------
// formatDate
// ---------------------------------------------------------------------------
describe("formatDate", () => {
  it("returns dash for null", () => {
    expect(formatDate(null)).toBe("-");
  });

  it("returns dash for empty string", () => {
    expect(formatDate("")).toBe("-");
  });

  it("displays the correct calendar day for a date-only string (timezone safety)", () => {
    // ISO date-only strings are UTC midnight by spec. Without the local-time
    // normalisation fix, US-timezone environments would render the previous day.
    const result = formatDate("2026-01-15");
    // The day number 15 must appear; 14 must not (it was the pre-fix off-by-one).
    expect(result).toContain("15");
    // Ensure the year round-trips correctly.
    expect(result).toContain("2026");
  });

  it("handles a full ISO datetime string", () => {
    const result = formatDate("2026-06-01T12:00:00Z");
    expect(result).not.toBe("-");
  });
});

// ---------------------------------------------------------------------------
// formatScore
// ---------------------------------------------------------------------------
describe("formatScore", () => {
  it("prefixes positive scores with +", () => {
    expect(formatScore("0.42")).toBe("+0.42");
    expect(formatScore("1")).toBe("+1.00");
  });

  it("keeps the minus sign for negative scores", () => {
    expect(formatScore("-0.42")).toBe("-0.42");
    expect(formatScore("-1")).toBe("-1.00");
  });

  it("formats zero without a sign", () => {
    expect(formatScore("0")).toBe("0.00");
    expect(formatScore("0.00")).toBe("0.00");
  });

  it("passes through non-numeric strings unchanged", () => {
    expect(formatScore("bad")).toBe("bad");
  });
});

// ---------------------------------------------------------------------------
// scoreClass
// ---------------------------------------------------------------------------
describe("scoreClass", () => {
  it("returns positive for scores above the neutral band", () => {
    expect(scoreClass("0.5")).toBe("positive");
    expect(scoreClass("0.06")).toBe("positive"); // just above +0.05
  });

  it("returns negative for scores below the neutral band", () => {
    expect(scoreClass("-0.5")).toBe("negative");
    expect(scoreClass("-0.06")).toBe("negative"); // just below -0.05
  });

  it("returns neutral for scores strictly within ±0.05", () => {
    expect(scoreClass("0")).toBe("neutral");
    expect(scoreClass("0.04")).toBe("neutral");
    expect(scoreClass("-0.04")).toBe("neutral");
  });

  it("treats ±0.05 as non-neutral (boundary is exclusive)", () => {
    expect(scoreClass("0.05")).toBe("positive");
    expect(scoreClass("-0.05")).toBe("negative");
  });

  it("returns neutral for non-numeric input", () => {
    expect(scoreClass("bad")).toBe("neutral");
  });
});

// ---------------------------------------------------------------------------
// scorePct
// ---------------------------------------------------------------------------
describe("scorePct", () => {
  it("maps -1 → 0, 0 → 50, +1 → 100", () => {
    expect(scorePct("-1")).toBe(0);
    expect(scorePct("0")).toBe(50);
    expect(scorePct("1")).toBe(100);
  });

  it("clamps values above 1 to 100 (prevents indicator escaping the meter bar)", () => {
    expect(scorePct("1.5")).toBe(100);
    expect(scorePct("99")).toBe(100);
  });

  it("clamps values below -1 to 0", () => {
    expect(scorePct("-1.5")).toBe(0);
    expect(scorePct("-99")).toBe(0);
  });

  it("returns 50 for non-numeric input", () => {
    expect(scorePct("bad")).toBe(50);
  });
});

// ---------------------------------------------------------------------------
// trimText
// ---------------------------------------------------------------------------
describe("trimText", () => {
  it("collapses internal whitespace to a single space", () => {
    expect(trimText("hello   world")).toBe("hello world");
    expect(trimText("a\t\nb")).toBe("a b");
  });

  it("trims leading and trailing whitespace", () => {
    expect(trimText("  hello  ")).toBe("hello");
  });

  it("leaves text at exactly 120 characters unchanged", () => {
    const exactly120 = "a".repeat(120);
    expect(trimText(exactly120)).toBe(exactly120);
  });

  it("truncates text longer than 120 characters with an ellipsis", () => {
    const long = "a".repeat(130);
    const result = trimText(long);
    expect(result).toHaveLength(120);
    expect(result).toMatch(/\.\.\.$/);
  });

  it("truncates at 121 characters (boundary)", () => {
    const justOver = "a".repeat(121);
    expect(trimText(justOver)).toHaveLength(120);
  });

  it("handles an empty string", () => {
    expect(trimText("")).toBe("");
  });
});
