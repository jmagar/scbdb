# Phase 1: Code Quality & Architecture Review

## Code Quality Findings

**33 findings** — 4 Critical, 8 High, 12 Medium, 9 Low

### Critical (4)

| # | Category | File | Summary |
|---|----------|------|---------|
| CQ-1 | Security | `middleware.rs:76` | Plaintext API key comparison — `HashSet::contains` is not constant-time; timing side-channel attack on auth. Fix: hash keys with SHA-256 on load, compare with `subtle::ConstantTimeEq`. |
| CQ-2 | Performance | `locator/fetch.rs:44` | `reqwest::Client` built per-attempt in retry loop — initializes new connection pool, TLS, DNS per call. 30+ brands × 3 retries = hundreds of throwaway pools. Fix: accept shared `&reqwest::Client`. |
| CQ-3 | Reliability | `middleware.rs:92` | Rate-limit `HashMap` grows unbounded — never pruned. DoS vector via spoofed `X-Forwarded-For`. Fix: add eviction when `state.len() > 10_000` or use TTL-aware map. |
| CQ-4 | Security | `api/mod.rs:111` | CORS allows `Any` origin — any website can make authenticated cross-origin requests. Fix: restrict to `SCBDB_CORS_ORIGINS` env var, default to `http://localhost:5173`. |

### High (8)

| # | Category | File | Summary |
|---|----------|------|---------|
| CQ-5 | Duplication | `locator/mod.rs:252-290` | URL resolution logic duplicated across `extract_store_locator_page_url` and `extract_dealers_page_url` |
| CQ-6 | Complexity | `locator/mod.rs:46-250` | `fetch_store_locations` is 250 lines, suppresses clippy lint, cyclomatic complexity ~30 |
| CQ-7 | Error Handling | `brand_newsroom/mod.rs:159` | Silent failure on homepage fetch — no logging on network errors or non-success status |
| CQ-8 | Duplication | `brand_completeness.rs:112-261` | 30-line completeness CTE SQL duplicated across two functions |
| CQ-9 | Technical Debt | `dashboard-page.tsx:77-118` | Duplicate query declarations — inline useQuery duplicates existing hooks in `use-dashboard-data.ts` |
| CQ-10 | Maintainability | `client.ts:78-103,139-164` | Error parsing logic duplicated between `apiGet` and `apiMutate` |
| CQ-11 | Security | `brand_intel.rs:110` | Env vars (`TEI_URL`, `YOUTUBE_API_KEY`) read at job execution time, not startup — misconfig surfaces hours after deploy |
| CQ-12 | Reliability | `locator/fetch.rs:25` | `curl` subprocess dependency without startup check — silent quality degradation without `curl` |

### Medium (12)

| # | Summary |
|---|---------|
| CQ-13 | Zod schemas defined but never used for runtime validation (`schemas.ts`) |
| CQ-14 | 80-line re-export block in `scbdb-db/src/lib.rs` |
| CQ-15 | `connect_or_exit` matches on unreachable `DbError` variants |
| CQ-16 | Six identical loading/error/empty patterns in `brand-recon-tab.tsx` |
| CQ-17 | New `reqwest::Client` per brand newsroom crawl |
| CQ-18 | `BrandOutcome::Ok { succeeded: false }` is confusing API |
| CQ-19 | Hardcoded Chrome UA version 124 will become stale |
| CQ-20 | YouTube `next_page_token` deserialized but unused (dead code) |
| CQ-21 | Error code to HTTP status mapping uses string matching |
| CQ-22 | 13 locator format modules with no trait abstraction |
| CQ-23 | `QueryClient` created at module scope (HMR stale cache risk) |
| CQ-24 | Test seed helpers use raw SQL instead of domain functions |

### Low (9)

CQ-25 through CQ-33: `#[allow(clippy::too_many_lines)]` annotations, loose string types in TS, `normalize_limit` takes `i64` instead of `u32`, unnecessary `Box::pin`, no container resource limits, deeply nested conditional rendering, unsafe `env::remove_var` in test, `fetch_text`/`fetch_json` don't share client, unsafe type cast `undefined as TResponse & void`.

---

## Architecture Findings

**9 findings** — 0 Critical, 3 Medium, 6 Low

### Architectural Strengths (Preserved)

- **S-01**: Clean dependency layering — acyclic, unidirectional. `scbdb-core` at bottom, binary crates at top.
- **S-02**: Consistent `thiserror` error handling across all crates with proper `#[source]`/`#[from]` chaining.
- **S-03**: Fail-open signal collection — individual source failures logged and skipped, never abort pipeline.
- **S-04**: Config injection via `Fn(&str) -> Result<String, VarError>` — hermetic testing without env var races.
- **S-05**: Uniform REST API envelope (`{ data, meta }` / `{ error, meta }`), cursor pagination, proper HTTP verbs.
- **S-06**: Frontend data layer properly separated: types → API client → TanStack Query hooks → components.
- **S-07**: Secret redaction in `AppConfig::Debug`.
- **S-08**: All-or-nothing Shopify collection prevents incorrect delta calculations from partial data.

### Medium (3)

| # | Category | Summary |
|---|----------|---------|
| AR-1 | Duplication | Location pipeline `RawStoreLocation → NewStoreLocation` conversion duplicated between CLI (`raw_to_new_location`) and scheduler (`scheduler/mod.rs:149-165`) with behavioral divergence — CLI defaults country to "US", scheduler passes through raw value. |
| AR-2 | Boundaries | `scbdb-profiler` takes `&PgPool` directly, breaking library purity pattern. Every other lib crate returns results to callers for persistence. Profiler fuses collection and DB writes. |
| AR-3 | Scalability | In-memory rate limiter with no eviction, no horizontal scaling path. Single-instance constraint undocumented. |

### Low (6)

| # | Summary |
|---|---------|
| AR-4 | String-based error code dispatch in `ApiError` — no compile-time exhaustiveness checking |
| AR-5 | Middleware errors use `MiddlewareErrorBody` (no `meta`/`request_id`), application errors use `ApiError` (with `meta`) — two error shapes |
| AR-6 | CORS allows any origin (pre-production acceptable, production risk) |
| AR-7 | Zod schemas defined but not applied — frontend trusts server responses without runtime validation |
| AR-8 | Divergent TEI env var names: `SENTIMENT_TEI_URL` vs `TEI_URL` |
| AR-9 | `scbdb-legiscan` normalization types not in shared `scbdb-core` (reasonable for single-source, limits future extensibility) |

---

## Critical Issues for Phase 2 Context

These findings should inform the Security and Performance review:

### Security-Relevant
1. **CQ-1**: Timing side-channel on API key comparison
2. **CQ-4/AR-6**: CORS allows any origin
3. **CQ-11**: Late env var reads in scheduler (misconfig surfaces at runtime, not startup)
4. **AR-5**: Middleware error responses lack request_id for tracing

### Performance-Relevant
1. **CQ-2**: `reqwest::Client` built per-attempt in hot loops (locator fetcher)
2. **CQ-3/AR-3**: Unbounded rate-limit HashMap (memory leak + DoS vector)
3. **CQ-8**: Duplicated completeness CTE SQL (maintenance/correctness risk, not perf per se)
4. **CQ-17**: New client per brand newsroom crawl

### Architectural
1. **AR-1**: Location pipeline behavioral divergence (country defaulting)
2. **AR-2**: Profiler DB coupling breaks testability
