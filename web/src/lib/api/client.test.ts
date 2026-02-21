import { describe, expect, it } from "vitest";

import { withQuery } from "./client";

describe("withQuery", () => {
  it("omits undefined query values", () => {
    expect(withQuery("/api/v1/products", { limit: 50, tier: undefined })).toBe(
      "/api/v1/products?limit=50",
    );
  });

  it("returns path when query is empty", () => {
    expect(withQuery("/api/v1/health", {})).toBe("/api/v1/health");
  });
});
