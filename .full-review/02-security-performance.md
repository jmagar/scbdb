# Phase 2: Security & Performance Review

## Security Findings

**15 findings** — 2 Critical, 3 High, 6 Medium, 4 Low

### Critical (2)

| # | CVSS | CWE | File | Summary |
|---|------|-----|------|---------|
| SEC-C1 | 7.5 | CWE-208 | `middleware.rs:76` | Timing side-channel on API key comparison — `HashSet::contains` not constant-time. Fix: SHA-256 hash keys on load, compare with `subtle::ConstantTimeEq`. |
| SEC-C2 | 8.1 | CWE-942 | `api/mod.rs:111` | CORS wildcard origin with authenticated endpoints — any website can make cross-origin API calls. One config line from catastrophic if `allow_credentials(true)` ever added. Fix: restrict to `SCBDB_CORS_ORIGINS` env var. |

### High (3)

| # | CVSS | CWE | File | Summary |
|---|------|-----|------|---------|
| SEC-H1 | 7.5 | CWE-770 | `api/mod.rs:218` | No request body size limit — multi-GB POST can OOM the server. Fix: `DefaultBodyLimit::max(1_048_576)`. |
| SEC-H2 | 7.5 | CWE-400 | `middleware.rs:195` | Rate limiter HashMap never evicts — memory exhaustion via spoofed `X-Forwarded-For`. Fix: periodic eviction + max entries cap. |
| SEC-H3 | 8.1 | CWE-78 | `locator/fetch.rs:25` | `curl` subprocess has no protocol restriction — SSRF via `file://`, `gopher://`, internal network URLs from user-controlled `store_locator_url`. Fix: `--proto =https,http` + `--max-filesize` + URL validation at API boundary. |

### Medium (6)

| # | Summary |
|---|---------|
| SEC-M1 | Missing security headers (X-Content-Type-Options, X-Frame-Options, CSP, HSTS) |
| SEC-M2 | Env vars read at job execution time, not startup (misconfig surfaces hours later) |
| SEC-M3 | Middleware errors lack request_id — security events can't be correlated with traces |
| SEC-M4 | No HTTPS enforcement — API keys transmitted in cleartext |
| SEC-M5 | Request ID spoofable from client — no format/length validation on `x-request-id` |
| SEC-M6 | Missing validation on enrichment write endpoints — no length limits on profile, social, domains |

### Low (4)

| # | Summary |
|---|---------|
| SEC-L1 | API keys stored in plaintext in memory (addressed by C1 remediation) |
| SEC-L2 | Hardcoded test credentials in CI |
| SEC-L3 | Default "changeme" password in `.env.example` |
| SEC-L4 | No `cargo audit` / `pnpm audit` in CI pipeline |

### Positive Security Observations

- Parameterized SQL everywhere — zero SQL injection surface
- `unsafe_code = "forbid"` at workspace level
- Auth required outside development mode
- Secrets properly gitignored
- Soft-delete pattern preserves audit trail
- Graceful shutdown for in-flight requests
- Scraper trust validation gate before persisting data
- Non-default PostgreSQL port (15432)

---

## Performance Findings

**23 findings** — 4 Critical, 6 High, 8 Medium, 5 Low

### Critical (4)

| # | File | Impact | Summary |
|---|------|--------|---------|
| PERF-1 | `locator/fetch.rs:44` | 7-15s wasted per sweep | `reqwest::Client` constructed per-call — hundreds of TLS/DNS/pool instantiations per locator run. Fix: shared `&reqwest::Client`. |
| PERF-2 | `locations/write.rs:28` | 500ms-1s per brand | N+1 DB writes — one `INSERT...ON CONFLICT` per location. Fix: batch with `UNNEST` for single round-trip. |
| PERF-3 | `middleware.rs:88` | Memory leak + contention | Unbounded rate-limit HashMap, never pruned. Mutex serializes all requests. Fix: background pruner + consider `DashMap`. |
| PERF-4 | `locations.rs:70` | 3-5MB payload at 10K locs | `/api/v1/locations/pins` returns all locations with no LIMIT. Fix: viewport-based spatial filtering or cursor pagination. |

### High (6)

| # | Impact | Summary |
|---|--------|---------|
| PERF-5 | 4x latency | `GET /brands/:slug` runs 4 DB queries sequentially. Fix: `tokio::try_join!` (already used in list endpoint). |
| PERF-6 | N correlated subqueries | `view_products_dashboard` LATERAL subquery per product row. Fix: `DISTINCT ON` CTE. |
| PERF-7 | 9N index probes | `get_all_brands_completeness` runs 9 correlated EXISTS per brand. Fix: LEFT JOIN + DISTINCT subqueries. |
| PERF-8 | 5 connections per load | Dashboard fires 5 parallel API calls on mount. Fix: combined summary endpoint. |
| PERF-9 | Unbounded result set | Products default limit is `i64::MAX`. Fix: apply `normalize_limit` unconditionally. |
| PERF-10 | Linear brand processing | Scheduler processes brands sequentially. Fix: `buffer_unordered(4)`. |

### Medium (8)

| # | Summary |
|---|---------|
| PERF-11 | No HTTP response compression — large JSON payloads sent uncompressed |
| PERF-12 | MapLibre GL (~600KB) loaded eagerly — should use `React.lazy` |
| PERF-13 | QueryClient default config — no explicit `gcTime`, `refetchOnWindowFocus` |
| PERF-14 | Connection pool default of 10 too low for dashboard + scheduler |
| PERF-15 | `price_snapshots` lacks partition strategy (forward-looking) |
| PERF-16 | `sentiment_snapshots` — monitor index usage |
| PERF-17 | `view_pricing_summary` GROUP BY includes unnecessary `deleted_at` columns |
| PERF-18 | 4 duplicate signal pagination query variants |

### Low (5)

PERF-19 through PERF-23: curl subprocess overhead (justified), duplicate lowercase on HTML body, GeoJSON rebuilt on reference change, no Cache-Control headers, correlated subquery in locations summary.

### Missing Indexes (Recommended)

| Table | Index | Justification |
|-------|-------|---------------|
| `store_locations` | `(latitude, longitude) WHERE is_active AND latitude IS NOT NULL` | Spatial queries for viewport filtering |
| `brands` | `(is_active, deleted_at) WHERE is_active = true AND deleted_at IS NULL` | Universal filter pattern |
| `collection_runs` | `(created_at DESC)` | "Most recent run" lookups |
| `brand_social_handles` | `(brand_id, is_active)` | Completeness queries |

---

## Critical Issues for Phase 3 Context

### Testing Implications
1. **SEC-H3**: URL validation at API boundary needs test coverage for all protocol schemes
2. **SEC-M6**: Enrichment endpoint validation needs boundary tests (max lengths, invalid formats)
3. **PERF-2**: Batch upsert needs integration test with large location sets
4. **PERF-4**: Pagination endpoint needs tests for cursor/viewport edge cases

### Documentation Implications
1. **SEC-C2**: CORS configuration needs documentation in deployment guide
2. **SEC-M4**: TLS termination architecture needs documentation
3. **PERF-14**: Connection pool sizing guidance for different workloads
4. Missing indexes should be tracked in a migration plan
