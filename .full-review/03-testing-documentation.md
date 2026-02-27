# Phase 3: Testing & Documentation Review

## Test Coverage Findings

### Current Test Inventory

**Rust Tests:**
- `crates/scbdb-core/src/brands_test.rs` — Brand config parsing
- `crates/scbdb-core/src/config_test.rs` — App config validation
- `crates/scbdb-scraper/src/client_test.rs` — Shopify client mocking
- `crates/scbdb-scraper/src/normalize_test.rs` — Product normalization
- `crates/scbdb-scraper/src/parse_test.rs` — HTML/JSON parsing
- `crates/scbdb-legiscan/src/client_test.rs` — LegiScan client
- `crates/scbdb-legiscan/tests/client.rs` — Integration tests
- `crates/scbdb-scraper/tests/shopify_client.rs` — Integration tests
- `crates/scbdb-db/tests/integration.rs` — DB integration tests
- `crates/scbdb-db/tests/live.rs` — Live DB tests
- `crates/scbdb-server/src/api/mod.rs` — `#[cfg(test)]` module with integration tests
- `crates/scbdb-server/src/middleware.rs` — `#[cfg(test)]` module with unit tests
- `crates/scbdb-cli/src/tests.rs` — CLI tests
- `crates/scbdb-cli/src/collect/brand_test.rs` — Brand collection tests
- `crates/scbdb-cli/src/collect/collect_test.rs` — Collection runner tests

**TypeScript Tests:**
- `web/src/smoke.test.ts` — Basic smoke test
- `web/src/lib/api/client.test.ts` — API client tests
- `web/src/lib/brand-colors.test.ts` — Color generation tests
- `web/src/components/dashboard-page.test.tsx` — Dashboard tests
- `web/src/components/brands-page.test.tsx` — Brands page tests
- `web/src/components/brand-profile-page.test.tsx` — Profile page tests
- `web/src/components/brand-signal-feed.test.tsx` — Signal feed tests
- `web/src/components/location-map-view.test.tsx` — Map view tests
- `web/src/components/map-filter-sidebar.test.ts` — Filter tests
- `web/src/components/dashboard-utils.test.ts` — Utility tests

### Findings

| # | Severity | Finding |
|---|----------|---------|
| T-1 | Critical | **No tests for security fixes**: Timing-safe auth comparison, CORS restriction, body size limit, rate limiter eviction, request ID validation, enrichment validation — all newly added code with zero test coverage |
| T-2 | High | **No tests for batch UNNEST upsert**: The `upsert_store_locations` rewrite from N+1 to batch has no integration test verifying correctness (new rows, updated rows, conflict handling) |
| T-3 | High | **No tests for shared reqwest::Client**: 15 format modules changed to accept `&Client` — no test verifies the client is properly threaded through |
| T-4 | High | **Locator pipeline untested**: `fetch_store_locations` (250 lines, 13 strategies) has zero unit or integration tests — relies entirely on manual testing |
| T-5 | Medium | **Sentiment pipeline untested**: `scbdb-sentiment` has no test files at all |
| T-6 | Medium | **Profiler untested**: `scbdb-profiler` has no test files |
| T-7 | Medium | **Frontend test coverage sparse**: Only 10 test files for 20+ components; no tests for ReconList refactor, lazy MapLibre, or new union types |
| T-8 | Medium | **No E2E tests**: No Playwright or browser-level testing |
| T-9 | Low | **Test pyramid imbalance**: Heavy on integration tests (sqlx::test), light on unit tests for pure logic |
| T-10 | Low | **CI runs no coverage reporting**: No `cargo-tarpaulin` or `vitest --coverage` |

### Recommendations

**T-1 (Critical):** Add tests for all security fixes:
```rust
#[test]
fn auth_state_timing_safe_comparison() {
    // Verify allows() works with valid key
    // Verify allows() rejects invalid key
    // Verify constant-time: both paths take similar duration
}

#[test]
fn cors_restricts_to_configured_origins() {
    // Set SCBDB_CORS_ORIGINS and verify only those are allowed
}

#[test]
fn body_size_limit_rejects_oversized_request() {
    // Send >1MB body, expect 413
}

#[test]
fn rate_limiter_evicts_stale_entries() {
    // Fill >1000 entries, advance time, verify pruning
}
```

---

## Documentation Findings

**Overall Score: 7/10** — Strong architecture docs, targeted gaps in API accuracy and frontend docs.

| # | Severity | Finding |
|---|----------|---------|
| D-1 | Critical | **API_DESIGN.md path parameter wrong**: Documents `{brand_id}` (UUID) but implementation uses `{slug}` (string) |
| D-2 | High | **DEPLOYMENT.md is draft**: Missing Docker builds, static assets, production secrets |
| D-3 | High | **Frontend zero doc comments**: React components and API client have no JSDoc |
| D-4 | Medium | **No JSON request/response examples** in API documentation |
| D-5 | Medium | **No architecture diagram**: System topology not visually documented |
| D-6 | Medium | **Server CLAUDE.md stale**: Still references `MiddlewareErrorBody` and `HashSet<String>` auth — both replaced by our fixes |
| D-7 | Low | **No migration index**: Readers must scan SQL files to understand schema evolution |
| D-8 | Low | **Module-level `//!` doc comments sparse**: ~30% of Rust modules have them |

### Strengths
- Per-crate CLAUDE.md files (8 total, 190-381 lines each) — exceptional
- DATABASE_SCHEMA.md comprehensive and current
- CLI reference complete with flags and examples
- .env.example thorough (99 lines, well-commented)
