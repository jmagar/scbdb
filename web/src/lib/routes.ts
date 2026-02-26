export const ROUTES = {
  dashboard: "#/",
  brands: "#/brands",
  brand: (slug: string) => `#/brands/${encodeURIComponent(slug)}`,
} as const;
