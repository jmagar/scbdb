# Phase 4: Best Practices & Standards

## Framework & Language Findings

| # | Severity | Finding |
|---|----------|---------|
| BP-1 | Medium | **`.any()` short-circuit in timing-safe auth**: The `allows()` method uses `.any(|stored| stored.ct_eq(&token_hash).into())` which short-circuits on first match. While `ct_eq` itself is constant-time, `.any()` returns early, leaking whether the match was early or late in the list. Fix: use `.fold(false, |acc, stored| acc | bool::from(stored.ct_eq(&token_hash)))` |
| BP-2 | Medium | **API key hashing without salt**: SHA-256 without salt is vulnerable to rainbow table attacks on weak tokens. Consider HMAC-SHA256 with a server-side salt from `SCBDB_API_KEY_HASH_SALT` env var |
| BP-3 | Medium | **No structured error details field**: `ErrorBody` has `code` and `message` but no `details` for field-level validation errors. Enrichment validation returns generic messages |
| BP-4 | Medium | **Scheduler handle lifetime fragile**: `let _scheduler = ...` could be silently dropped in future refactors. Consider a `SchedulerGuard` newtype |
| BP-5 | Low | **Zod error response validation**: Frontend `throwApiError` parses error bodies with loose type checks instead of Zod schema validation |
| BP-6 | Low | **No viewport meta tag**: `web/index.html` may be missing proper mobile viewport configuration |
| BP-7 | Low | **No scheduler health endpoint**: Cannot monitor if background jobs are running or failing |

### Strengths
- Modern Rust (edition 2021, async/await throughout)
- Compile-time SQL verification via sqlx
- Current dependencies: tokio 1.49, axum 0.8, sqlx 0.8, React 19, TanStack Query 5, Zod 4.3
- Strict TypeScript mode enabled
- `unsafe_code = "forbid"` at workspace level
- Clean middleware composition with Tower ServiceBuilder

---

## CI/CD & DevOps Findings

| # | Severity | Finding |
|---|----------|---------|
| CI-1 | Medium | **Web build not cached in CI**: Vite output not cached between runs |
| CI-2 | Medium | **No SBOM generation**: No CycloneDX or SPDX supply chain visibility |
| CI-3 | Low | **No coverage reporting**: Neither `cargo-tarpaulin` nor `vitest --coverage` runs in CI |
| CI-4 | Low | **No scheduler observability**: No health endpoint for background job monitoring |

### Strengths
- Three-stage CI pipeline (check → test → web) with proper dependency ordering
- Cargo audit + pnpm audit now in CI (added by our fixes)
- Lefthook git hooks: pre-commit (fmt), pre-push (test + clippy)
- Comprehensive justfile with 20+ commands
- Alpine PostgreSQL with health checks
- Non-default port (15432) reduces exposure
- Docker resource limits on postgres (added by our fixes)
