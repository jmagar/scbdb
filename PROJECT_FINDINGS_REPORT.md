# SCBDB Investigation & Status Report

- Date: 2026-02-25
- Scope audited: docs, backend/workspace code, frontend code, migrations, tests, and live runtime behavior
- Audit mode: systematic plan-vs-implementation comparison + live verification

## 1) Audit Coverage

The following were reviewed in detail:

- Core scope docs:
  - `README.md`
  - `docs/PRD.md`
  - `docs/MVP.md`
  - `docs/mvp_phases/phase-1-foundation.md`
  - `docs/mvp_phases/phase-2-collection-cli.md`
  - `docs/mvp_phases/phase-3-regulatory-tracking.md`
  - `docs/mvp_phases/phase-4-sentiment-pipeline.md`
  - `docs/mvp_phases/phase-5-api-dashboard.md`
- Supporting docs:
  - `docs/ARCHITECTURE.md`
  - `docs/API_DESIGN.md`
  - `docs/TESTING.md`
  - `docs/DEPLOYMENT.md`
  - `docs/STORE_LOCATOR.md`
  - `docs/LOCATIONS_DASHBOARD.md`
  - `docs/BRAND_INTELLIGENCE.md`
  - `docs/SENTIMENT_DASHBOARD.md`
- Codebase surfaces:
  - Rust workspace crates (`scbdb-cli`, `scbdb-server`, `scbdb-core`, `scbdb-db`, `scbdb-scraper`, `scbdb-legiscan`, `scbdb-sentiment`, `scbdb-profiler`)
  - Frontend app (`web/src`)
  - Migrations (`migrations/*.sql`)
  - Test suites (Rust + web)
- Live command and runtime validation executed:
  - `just test`
  - `just check`
  - `cargo run --bin scbdb-server`
  - `pnpm --dir web build`
  - `pnpm --dir web preview --host 0.0.0.0 --port 4173`
  - Chrome DevTools MCP UI+network validation

## 2) What Is Implemented (Confirmed in Code)

### Workspace and platform foundations

- Rust Cargo workspace with expected crate boundaries exists and is active.
- CLI and server binaries are present and wired.
- PostgreSQL migration set exists and is extensive.
- `config/brands.yaml` single brand registry model is present and used.

### Backend/API implementation

- API router includes system + domain routes for products, pricing, bills, sentiment, locations, and brand intelligence under `/api/v1`.
- Middleware exists for request IDs, bearer auth behavior, and rate limiting.
- DB layer is modular and implements typed query modules for major domains.
- Brand intelligence read/write APIs are implemented (list/detail/signals/intel writes).
- Scheduler code exists for periodic tasks (including location-related scheduling).

### CLI implementation

- `db` commands implemented (`ping`, `migrate`, `seed`).
- `collect` command family implemented (products/pricing/verify-images/locations).
- `regs` command family implemented (ingest/status/timeline/report).
- `sentiment` command family implemented (collect/status/report).

### Data collection and enrichment implementation

- Shopify scraping and normalization path exists.
- Regulatory ingestion client integration exists.
- Sentiment pipeline crate exists with sources/scorer/vector store components.
- Store locator detection/extraction stack is implemented across many locator formats.
- Brand profiling/intelligence ingestion infrastructure exists.

### Frontend implementation

- Vite + React app exists and builds.
- Dashboard page with 5 tabs exists: Products, Pricing, Bills, Sentiment, Locations.
- Brands registry page and brand profile page are implemented.
- Brand profile has feed/content/recon/edit surfaces and associated hooks/API clients.

## 3) Planned Scope vs Delivered Scope

### Phases status (from docs vs code reality)

- Phase 1 (Foundation): implemented.
- Phase 2 (Collection CLI): implemented.
- Phase 3 (Regulatory): implemented.
- Phase 4 (Sentiment pipeline): implemented (core structure + tests present).
- Phase 5 (API/dashboard): substantially implemented.

### Documented-but-missing or partially missing

- API design includes `GET /collection-runs`, but router currently does not expose it.
- API design documents OpenAPI endpoint `/api/v1/openapi.json`; not implemented.
- API design documents idempotency key behavior for writes; not implemented/enforced.
- CLI top-level `report` is still a stub and exits as not implemented.

## 4) Live Verification Results (Reality Check)

### Rust tests (`just test`)

- Rust side is green.
- Total observed: 511 passed, 0 failed, 2 ignored.
- Coverage breadth includes CLI parsing, core config/brands, DB live tests, scraper, legiscan, profiler, sentiment, and server tests.

### Web tests (`just test`)

- Web test run fails.
- Summary observed: 8 failing tests + 2 failing suites.
- Primary failures:
  - `web/src/components/brand-signal-feed.test.tsx`: repeated `Cannot read properties of undefined (reading 'flatMap')` due to test data shape mismatch with `data?.pages.flatMap(...)` usage in component.
  - `web/src/components/brand-profile-page.test.tsx`: `document is not defined`.
  - `web/src/components/brands-page.test.tsx`: `document is not defined`.

### Quality gate (`just check`)

- Fails on clippy error:
  - `crates/scbdb-db/src/locations/write.rs`
  - `upsert_store_locations` flagged by `clippy::too_many_lines` (117/100) under `-D warnings`.

### Runtime startup

- Server startup currently blocked:
  - `cargo run --bin scbdb-server` returns
  - `Error: migration 20260221000200 was previously applied but has been modified`
- This is a critical operational issue; backend cannot boot cleanly against current DB migration history.

### Frontend runtime via Chrome DevTools MCP

- Frontend shell renders (tabs/header/navigation present).
- Data calls fail across tabs due to backend unavailability.
- Observed network failures:
  - `/api/v1/products`, `/pricing/summary`, `/bills`, `/sentiment/summary`, `/locations/*`, `/brands` returning failures when backend unavailable.
- Vite proxy logs repeatedly show:
  - `connect ECONNREFUSED 127.0.0.1:3000`

## 5) Everything Implemented but Not Currently Working

1. End-to-end dashboard data loading is not working in current runtime due to backend startup failure and downstream API proxy refusals.
2. Web test suite is not healthy (brand-related tests/suites failing).
3. `just check` fails due to clippy gate on oversized function.
4. Server boot path is blocked by migration integrity mismatch.

## 6) Known Gaps / Drift Between Docs and Code

1. `GET /collection-runs` is documented but not routed.
2. OpenAPI endpoint is documented but absent.
3. Idempotency key semantics are documented but not enforced.
4. `scbdb-cli report` remains stubbed while reporting capability is documented as part of expected workflow.
5. Brand intelligence UX drift:
   - Docs describe richer Content/Recon expectations (including specific intel presentation emphasis), but current tab composition appears to differ from that narrative.

## 7) Duplicate Code / Maintainability Findings

1. Duplicated URL validation logic in brand write handlers:
   - `crates/scbdb-server/src/api/brands/write.rs`
   - `crates/scbdb-server/src/api/brands/write_enrichment.rs`
2. Redundant brand API helper in frontend (`fetchBrand` vs `fetchBrandProfile`) suggests API client surface can be tightened.
3. Some inline style-heavy components in map/filter UI could be normalized into reusable style utilities/classes.

## 8) Quick Wins (High ROI)

1. Fix migration integrity blocker first so server can start.
2. Fix web tests:
   - Make `brand-signal-feed` test fixtures match expected `pages` shape.
   - Resolve test environment/import pattern causing `document is not defined`.
3. Split/refactor `upsert_store_locations` to satisfy clippy line-limit gate.
4. Implement `GET /api/v1/collection-runs` using already-existing DB query layer.
5. Remove redundant frontend API helper and centralize duplicate URL validation helper on backend.

## 9) What Is Going Well

1. Scope breadth and implementation velocity are strong.
2. Rust backend and domain crates have substantial test depth and currently pass.
3. Architecture is modular with coherent crate boundaries.
4. Frontend structure (types/hooks/api/components) is organized and understandable.
5. Migrations and schema coverage are comprehensive for current product goals.

## 10) What Definitely Needs Work

1. Release reliability discipline around migrations (hash/immutability process).
2. Frontend test architecture consistency and runtime environment setup.
3. Contract alignment between docs and shipped API endpoints/features.
4. CI gate consistency so "green" actually means deployable end-to-end.
5. Better operational smoke path validating server boot + API probes before UI validation.

## 11) Actionable Plan to Get Back on Course

### Phase A — Restore operational baseline (immediate)

1. Resolve migration integrity mismatch (`20260221000200`) without rewriting applied migration history in-place.
2. Start backend successfully and verify:
   - `GET /api/v1/health`
   - representative read endpoints (`/products`, `/brands`, `/locations/summary`).
3. Re-run frontend against live backend to confirm data loading resumes.

### Phase B — Re-establish quality gates (same cycle)

1. Fix failing web tests (`brand-signal-feed`, brand page suites).
2. Refactor oversized DB function flagged by clippy.
3. Re-run and require both:
   - `just check` = pass
   - `just test` = pass

### Phase C — Close documented contract drift (next cycle)

1. Add `/api/v1/collection-runs` endpoint.
2. Decide and execute one of:
   - implement OpenAPI and idempotency-key behavior, or
   - explicitly downgrade docs from “active contract” to “planned” on those points.
3. Implement or explicitly scope/defer `scbdb-cli report` with updated docs.

### Phase D — Hardening and cleanup (follow-up)

1. Deduplicate validation helpers and tighten API client surface.
2. Align Brand Intelligence tabs with documented UX narrative.
3. Add startup smoke checks to CI/local workflows:
   - DB up -> migrate validation -> server boot -> minimal endpoint probes.

## 12) Evidence Summary (Command Outcomes)

- `just test`:
  - Rust tests all passed.
  - Web tests failed (8 tests + 2 suites).
- `just check`:
  - failed on clippy `too_many_lines` in `crates/scbdb-db/src/locations/write.rs`.
- `cargo run --bin scbdb-server`:
  - failed due to modified previously-applied migration `20260221000200`.
- `pnpm --dir web build`:
  - succeeded.
- Chrome DevTools MCP frontend inspection:
  - shell renders; tab-level API data fails when backend unavailable/proxy cannot connect.

## 13) Final Status Call

- Implementation progress: high.
- Test/quality maturity: mixed (Rust strong, web unstable).
- Runtime readiness: currently blocked by migration integrity issue.
- Immediate priority: restore backend startup + regain green test/check gates before feature expansion.

