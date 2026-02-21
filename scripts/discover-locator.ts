#!/usr/bin/env tsx
/**
 * Store Locator API Discovery Tool
 *
 * Loads a page in a headless browser, intercepts XHR/fetch responses, and
 * identifies JSON arrays that contain lat/lng-like fields.
 *
 * Usage:
 *   cd scripts
 *   pnpm install
 *   npx playwright install chromium --with-deps
 *   pnpm discover <url> [> ../config/locators/<brand>.json]
 *
 * Example:
 *   pnpm discover https://example.com/where-to-buy > ../config/locators/example.json
 */

import { chromium, type Request, type Response } from 'playwright';

const url = process.argv[2];
if (!url) {
  console.error('Usage: pnpm discover <store-locator-url>');
  process.exit(1);
}

function looksLikeLocationData(data: unknown): boolean {
  const arr =
    Array.isArray(data) ? data
    : (data as Record<string, unknown>)?.results
    ?? (data as Record<string, unknown>)?.locations
    ?? (data as Record<string, unknown>)?.stores
    ?? (data as Record<string, unknown>)?.data;

  if (!Array.isArray(arr) || arr.length === 0) return false;
  const first = arr[0] as Record<string, unknown>;
  const keys = Object.keys(first).map((k) => k.toLowerCase());
  return keys.some((k) =>
    ['lat', 'latitude', 'lng', 'longitude', 'lon', 'geo'].some((g) => k.includes(g))
  );
}

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();

const discovered: Array<{
  url: string;
  method: string;
  postData: string | null;
  sampleResponse: unknown;
}> = [];

page.on('response', async (response: Response) => {
  const req: Request = response.request();
  if (!['xhr', 'fetch'].includes(req.resourceType())) return;
  try {
    const body = await response.json();
    if (looksLikeLocationData(body)) {
      discovered.push({
        url: req.url(),
        method: req.method(),
        postData: req.postData(),
        sampleResponse: body,
      });
    }
  } catch {
    // Not JSON or not location-like — skip
  }
});

console.error(`Navigating to ${url} ...`);
await page.goto(url, { waitUntil: 'networkidle', timeout: 30_000 });
await page.waitForTimeout(3_000); // allow deferred XHR to fire after DOM settles

await browser.close();

if (discovered.length === 0) {
  console.error(
    'No location-like API calls detected.\n' +
      'Possible causes:\n' +
      '  1. Page requires a zip code search trigger before loading stores\n' +
      '  2. Store data is embedded in the HTML (JSON-LD or script tag)\n' +
      '  3. Anti-bot block — check the page in a real browser first'
  );
  process.exit(1);
}

// Write to stdout; redirect to file or pipe to jq
console.log(JSON.stringify(discovered, null, 2));
