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
] as const;

/**
 * Returns a deterministic hex color for a brand slug using djb2 hash.
 * Same slug always maps to the same color from the 12-color PALETTE.
 */
export function getBrandColor(slug: string): string {
  let hash = 0;
  for (const char of slug) {
    hash = (hash * 31 + char.charCodeAt(0)) & 0xffffffff;
  }
  // modulo guarantees index is always in [0, PALETTE.length); `!` is safe here
  return PALETTE[Math.abs(hash) % PALETTE.length]!;
}

/**
 * Builds a slug → color map for an array of brand slugs.
 */
export function getBrandColors(slugs: string[]): Record<string, string> {
  return Object.fromEntries(slugs.map((s) => [s, getBrandColor(s)]));
}
