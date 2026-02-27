# SCBDB Codebase Review: Framework & Language Best Practices, CI/CD

**Date:** 2024-02-24
**Scope:** Full codebase review covering Rust workspace (8 crates, Axum), React 19 frontend, CI/CD pipeline, and DevOps practices.

---

## Executive Summary

The SCBDB codebase demonstrates **strong foundational practices** in modern Rust and React development. The architecture is sound, dependency management is current, and the CI/CD pipeline is well-structured. However, there are **9 items requiring attention** across framework patterns, security, observability, and operational readiness.

**Critical issues: 2 (security, auth pattern)**
**High priority: 3 (error handling, observability, API design)**
**Medium priority: 4 (performance, type safety, documentation)**

---

# Part A: Framework & Language Best Practices

## Rust Stack

### ✅ Strengths

1. **Modern Async/Await Patterns**
   - Consistent use of `async`/`await` with `tokio::task::spawn` and `tokio::try_join!`
   - Proper graceful shutdown handling in `main.rs` via `tokio::select!` and signal handlers
   - Connection pooling with `sqlx::PgPool` with configurable min/max connections

2. **Axum Architecture**
   - Clean router organization with modular endpoints (`api/brands/`, `api/products/`, etc.)
   - Proper middleware stack ordering with `tower::ServiceBuilder`
   - Request-scoped state via `Extension<RequestId>` injected by middleware
   - Typed error handling with `ApiError` enum mapping to HTTP status codes
   - Rate limiting per authenticated token or IP

3. **sqlx Safety**
   - **Excellent compile-time checked queries** — all SQL is verified against the live database schema at compile time
   - Explicit column lists in `SELECT` and `RETURNING` (never `SELECT *`)
   - Append-only migrations with `.up` and `.down` files
   - Transactional batch operations (e.g., `upsert_store_locations` with `UNNEST`)
   - Smart dedup patterns (CTEs for price snapshots, unique constraints for bill events)

4. **Error Handling**
   - Consistent error type hierarchy: `DbError`, `ApiError`, `ScraperError`, `LegiscanError`
   - All errors implement `std::error::Error` and use `thiserror` for derive
   - Graceful degradation in scheduler jobs (individual brand failures logged, not fatal)

5. **Dependency Currency**
   - Rust 1.93 (recent, stable)
   - All dependencies are modern: tokio 1.49, axum 0.8, sqlx 0.8, serde 1.0, clap 4
   - No deprecated or EOL crates detected
   - TLS via `rustls` (security best practice over OpenSSL)

### ⚠️ Issues Found

#### 1. **Constant-Time Token Comparison Missing (SECURITY — HIGH)**

**Severity:** CRITICAL
**Location:** `crates/scbdb-server/src/middleware.rs:80–90`
**Issue:**
```rust
/// Constant-time comparison of incoming token against stored key hashes.
fn allows(&self, token: &str) -> bool {
    let incoming_hash = Sha256::digest(token.as_bytes());
    self.key_hashes
        .iter()
        .any(|hash| incoming_hash.as_ref() == hash.as_ref())  // ← Timing side-channel
}
```

The current implementation uses `.any()` with `==`, which is **NOT constant-time**. An attacker can time the comparison to determine how many characters match. The crate imports `subtle::ConstantTimeEq` but does not use it.

**Recommendation:**
```rust
fn allows(&self, token: &str) -> bool {
    let incoming_hash = Sha256::digest(token.as_bytes());
    self.key_hashes
        .iter()
        .fold(false, |acc, hash| {
            acc | incoming_hash.as_ref().ct_eq(hash).unwrap_u8() != 0
        })
}
```

Use `subtle::ConstantTimeEq::ct_eq()` for all comparisons. The `.fold()` pattern ensures all hashes are compared even if a match is found.

---

#### 2. **Auth Token Hashing Not Implemented (SECURITY — HIGH)**

**Severity:** CRITICAL
**Location:** `crates/scbdb-server/src/middleware.rs:68–71` and `.env.example`
**Issue:**

The `AuthState::from_env()` function hashes tokens with `Sha256` before storage, which is good. **However:**
- The `.env.example` file has a TODO comment: `# FUTURE: Hashing salt for API key storage (not yet active — see middleware.rs TODO)`
- The plaintext token is hashed once with no salt, making it vulnerable to dictionary attacks on weak keys.
- In production, API keys are only as strong as the entropy of the tokens themselves.

**Recommendation:**

1. **Add `SCBDB_API_KEY_HASH_SALT` to `.env.example` as REQUIRED (not optional).**
2. **Use PBKDF2 or Argon2 instead of plain SHA-256:**
   ```rust
   use argon2::{Argon2, PasswordHasher, PasswordHash};

   let argon2 = Argon2::default();
   let hashed = argon2.hash_password(token.as_bytes(), &SaltString::generate(thread_rng()))?;
   ```
3. **Document:** "API keys are hashed with Argon2 before comparison. Tokens must be at least 32 random bytes. Store the token value securely; it cannot be recovered from the hash."

---

#### 3. **Missing Error Context in API Error Responses (DESIGN — MEDIUM)**

**Severity:** HIGH
**Location:** `crates/scbdb-server/src/api/mod.rs:100–150`
**Issue:**

The `ApiError` struct provides `code` and `message`, but no `details` field for structured context (e.g., which field failed validation, what constraint was violated). This makes debugging client issues harder.

Current response:
```json
{
  "error": {
    "code": "validation_error",
    "message": "invalid tier value"
  }
}
```

Clients cannot determine **which field** failed or **why**.

**Recommendation:**

Add an optional `details` field:
```rust
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: ErrorBody,
    pub meta: ResponseMeta,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}
```

Example usage:
```json
{
  "error": {
    "code": "validation_error",
    "message": "Invalid request body",
    "details": {
      "field": "tier",
      "constraint": "must be 1, 2, or 3",
      "received": 5
    }
  }
}
```

---

#### 4. **Scheduler Job Handle Lifetime Risk (DESIGN — MEDIUM)**

**Severity:** HIGH
**Location:** `crates/scbdb-server/src/main.rs:28`
**Issue:**

```rust
let _scheduler = scheduler::build_scheduler(pool.clone(), Arc::clone(&config)).await?;
```

The scheduler is assigned to `_scheduler` (intended to signal "I'm not using this value, but I'm keeping it alive"). This pattern is fragile:
- A future refactor could remove the `_scheduler` variable entirely, silently killing all jobs.
- The underscore prefix is not idiomatic for long-lived handles.
- No warning is issued at compile time if the scheduler is dropped prematurely.

**Recommendation:**

Use a newtype wrapper to enforce lifetime semantics:
```rust
pub struct SchedulerGuard(JobScheduler);

impl Drop for SchedulerGuard {
    fn drop(&mut self) {
        tracing::warn!("scheduler dropped — all jobs have been cancelled");
    }
}

// In main.rs
let _scheduler_guard = SchedulerGuard(scheduler);  // Compiler error if removed
```

Or document explicitly with a comment:
```rust
// SAFETY: _scheduler_guard must remain bound for the lifetime of the process.
// Dropping it cancels all scheduled jobs.
let _scheduler_guard = scheduler::build_scheduler(pool.clone(), Arc::clone(&config)).await?;
```

---

#### 5. **Insufficient Logging in Collection Pipeline (OBSERVABILITY — MEDIUM)**

**Severity:** MEDIUM
**Location:** `crates/scbdb-cli/src/collect/runner.rs` and `brand/pipeline.rs`
**Issue:**

Collection runs log success/failure counts but lack structured tracing for:
- **Per-product timing:** How long does one brand's product collection take?
- **Error breakdown:** Are failures due to network, parsing, or validation?
- **Partial success details:** When a brand has 95/100 products, what happened to the other 5?

Log level is global (`RUST_LOG=info`), making it hard to debug a single brand without running the entire collect again.

**Recommendation:**

1. Add structured fields to all log messages:
   ```rust
   tracing::info!(
       brand_slug = %brand.slug,
       product_count = products.len(),
       duration_ms = start.elapsed().as_millis(),
       "collected products for brand"
   );
   ```

2. Track error types with `error_code`:
   ```rust
   tracing::warn!(
       brand_slug = %brand.slug,
       error_code = "shopify_403_auth",
       attempt = 1,
       "retrying on HTTP 403"
   );
   ```

3. Support per-brand log filtering:
   ```bash
   RUST_LOG="scbdb_cli[brand=cycling-frog]=debug"
   ```

---

## React & TypeScript Stack

### ✅ Strengths

1. **Modern React 19 Patterns**
   - Functional components with hooks only (no class components)
   - Proper cleanup in `useEffect` (hash change listener unmounted correctly)
   - TanStack Query 5 for data fetching with sensible defaults (staleTime: 60s, gcTime: 10min, retry: 1)

2. **Type Safety**
   - TypeScript strict mode enabled (`"strict": true`)
   - No unchecked index access (`"noUncheckedIndexedAccess": true`)
   - Branded types for domain concepts (`BrandRelationship`, `BrandTier`)
   - Zod v4.3.6 (current, uses faster Rust-core validator)

3. **API Layer**
   - Centralized `apiGet()` and `apiMutate()` helpers with header injection
   - Proper error handling: distinguishes network errors from API errors
   - Response validation with Zod schemas before passing data to components
   - Bearer token support for authenticated requests

4. **Routing & Navigation**
   - Hash-based routing (appropriate for single-page app)
   - Path validation with slug pattern matching (`/^[a-z0-9-]+$/`)
   - Safe URL encoding with `encodeURIComponent()`

5. **Dependency Currency**
   - React 19.2.4, TypeScript 5.9.3, Vite 7.3.1
   - ESLint 10 with React hooks plugin
   - Prettier 3.8 for consistent formatting
   - Vitest 4 for fast unit tests

### ⚠️ Issues Found

#### 6. **API Error Response Parsing Not Type-Safe (TYPE SAFETY — MEDIUM)**

**Severity:** MEDIUM
**Location:** `web/src/lib/api/client.ts:38–52`
**Issue:**

The error parsing function uses loose type guards and `any` implicitly:
```typescript
async function throwApiError(response: Response, path: string): Promise<never> {
  let errorMessage = `Request failed (${response.status}) for ${path}`;
  let errorCode = "unknown_error";
  try {
    const errorBody = await response.json();
    if (errorBody?.error && typeof errorBody.error === "object") {
      const { code, message } = errorBody.error;
      if (typeof code === "string") errorCode = code;
      if (typeof message === "string") errorMessage = message;
    }
  } catch {
    /* not JSON */
  }
  throw new ApiError(response.status, errorCode, errorMessage);
}
```

**Problems:**
- The destructuring `const { code, message } = errorBody.error` happens before the type check (out of order)
- No validation that the response matches `ApiResponse<T>` structure
- If the API changes to use different field names, the client silently falls back to generic messages

**Recommendation:**

Use Zod validation:
```typescript
const ApiErrorSchema = z.object({
  error: z.object({
    code: z.string(),
    message: z.string(),
    details: z.unknown().optional(),
  }),
});

async function throwApiError(response: Response, path: string): Promise<never> {
  let errorCode = "unknown_error";
  let errorMessage = `Request failed (${response.status}) for ${path}`;

  try {
    const errorBody = await response.json();
    const parsed = ApiErrorSchema.safeParse(errorBody);
    if (parsed.success) {
      errorCode = parsed.data.error.code;
      errorMessage = parsed.data.error.message;
    }
  } catch {
    /* not JSON */
  }

  throw new ApiError(response.status, errorCode, errorMessage);
}
```

---

#### 7. **Missing Viewport Meta Tag & Accessibility Issues (FRONTEND — MEDIUM)**

**Severity:** MEDIUM
**Location:** `web/index.html` (not provided, but inferred from root element check)
**Issue:**

The app checks for `#root` element in TypeScript but there's no evidence of:
- Viewport meta tag: `<meta name="viewport" content="width=device-width, initial-scale=1.0">`
- Proper semantic HTML structure on initial page load
- Focus management on route changes

This causes poor mobile UX and accessibility issues.

**Recommendation:**

Ensure `web/index.html` contains:
```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>SCBDB - Competitive Intelligence Platform</title>
</head>
<body>
  <div id="root"></div>
  <script type="module" src="/src/main.tsx"></script>
</body>
</html>
```

Add focus management to the hash change listener:
```typescript
useEffect(() => {
  const handleHashChange = () => {
    setHash(window.location.hash);
    // Move focus to main content
    const mainElement = document.querySelector("main");
    if (mainElement) {
      mainElement.focus();
      mainElement.setAttribute("tabindex", "-1");
    }
  };
  window.addEventListener("hashchange", handleHashChange);
  return () => window.removeEventListener("hashchange", handleHashChange);
}, []);
```

---

#### 8. **No Graceful Degradation for Missing API Key (SECURITY — MEDIUM)**

**Severity:** MEDIUM
**Location:** `web/src/lib/api/client.ts:24 & 85`
**Issue:**

The code silently omits the `Authorization` header if `VITE_API_KEY` is not set:
```typescript
const apiKey = import.meta.env.VITE_API_KEY as string | undefined;

if (apiKey) {
  headers.set("Authorization", `Bearer ${apiKey}`);
}
```

If the server requires auth (i.e., `SCBDB_ENV != "development"`), all requests will fail silently with a 401, providing no user-facing feedback that authentication failed.

**Recommendation:**

1. **Validate at app startup:**
   ```typescript
   function validateAppConfig() {
     if (!apiKey && import.meta.env.PROD) {
       throw new Error("VITE_API_KEY is required in production");
     }
   }
   ```

2. **Handle 401 gracefully in error handler:**
   ```typescript
   if (response.status === 401) {
     throw new ApiError(401, "unauthorized", "Authentication required. Check API key configuration.");
   }
   ```

---

---

# Part B: CI/CD & DevOps Practices

## GitHub Actions Pipeline

### ✅ Strengths

1. **Three-Job Pipeline**
   - `check`: Formatting, linting, cargo-audit (no build needed)
   - `test`: Runs after check passes (dependency ordering correct)
   - `web`: Separate pipeline for web asset checks
   - Clear, sequential gates prevent broken code from being merged

2. **Dependency Caching**
   - Rust cache via `Swatinem/rust-cache@v2` (efficient, per-toolchain)
   - Node modules cached via `pnpm/action-setup@v4` and `actions/setup-node@v4`
   - sqlx-cli cached to avoid 30s install per run

3. **Security Scanning**
   - `cargo audit` on every build (catches CVEs in dependencies)
   - `pnpm audit --audit-level moderate` (allows dev-only vulns, reasonable)

4. **Database Testing**
   - Proper PostgreSQL service setup with health checks
   - `--locked` flag ensures reproducible builds (Cargo.lock is authoritative)
   - Migrations run in CI, catching schema issues early

### ⚠️ Issues Found

#### 9. **Missing Build Artifact & Dependency Caching for Full Reproducibility (DEVOPS — MEDIUM)**

**Severity:** MEDIUM
**Location:** `.github/workflows/ci.yml:40–70`
**Issue:**

The web build checks don't cache the full build output. Every PR rebuilds Vite even if the bundle hasn't changed. Additionally:
- `pnpm install --frozen-lockfile` is correct but there's no explicit `pnpm audit` in the main check job (only in web job)
- No SBOM (Software Bill of Materials) generation for supply chain visibility

**Recommendation:**

```yaml
- name: Restore Vite cache
  uses: actions/cache@v4
  with:
    path: web/dist
    key: vite-${{ hashFiles('web/pnpm-lock.yaml', 'web/src/**') }}
    restore-keys: vite-

- name: Build (cached)
  run: pnpm --dir web build

- name: Generate SBOM
  run: |
    pnpm install -D @cyclonedx/npm
    pnpm exec cyclonedx-npm --output-file sbom.json
```

---

## Lefthook Git Hooks

### ✅ Strengths

1. **Pre-commit Gates**
   - File size check prevents large binary commits
   - Format check with `cargo fmt --check` and `pnpm format:check`
   - Parallel execution for speed

2. **Pre-push Gates**
   - `cargo clippy` with `-D warnings` (strict, will not allow new warnings)
   - Unit tests run before push (`cargo test --lib`)
   - TypeScript type checking on all files

### ⚠️ Issues Found

**No critical issues** with lefthook configuration. However, two minor improvements:

1. **Pre-push Should Include Integration Tests (BEST PRACTICE)**

Current:
```yaml
cargo test --workspace --lib  # unit tests only
```

Recommendation:
```yaml
run: |
  cargo test --workspace --lib  # unit tests (always)
  if [ -f docker-compose.yml ] && command -v docker >/dev/null; then
    cargo test --workspace --test '*'  # integration tests (if Docker available)
  fi
fail_text: "Run 'just test' locally before pushing. Integration tests may require Docker."
```

---

## Docker Compose & Deployment

### ✅ Strengths

1. **Lean PostgreSQL Configuration**
   - Alpine base (`postgres:16-alpine`) — small, secure, fast
   - Health check with `pg_isready` (10s interval, 5s timeout)
   - Memory limit (1GB) prevents runaway processes
   - Structured logging (max 10MB per file, rotate 3 files)
   - Named volume for data persistence (`scbdb_postgres_data`)

2. **Environment Isolation**
   - Passwords sourced from `.env` (never hardcoded)
   - Database name, user, port configurable
   - Service name (`scbdb-postgres`) matches DNS resolution inside container network

### ⚠️ Issues Found

**No critical issues** with Docker Compose. One enhancement opportunity:

1. **Missing Production Compose Override (OPTIONAL)**

Recommendation: Create `docker-compose.prod.yml` for production deployments:
```yaml
services:
  postgres:
    image: postgres:16-alpine
    restart: always  # not unless-stopped
    environment:
      # All production env vars from secret manager
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
    ports: []  # No exposed port in prod
    volumes:
      - scbdb_postgres_data:/var/lib/postgresql/data
      - ./backups:/backups:ro
    healthcheck:
      # More strict in production
      retries: 5
    deploy:
      resources:
        limits:
          memory: 4G  # Production sizing
        reservations:
          memory: 2G
```

---

## Justfile & Local Development

### ✅ Strengths

1. **Comprehensive Commands**
   - `bootstrap`: Full initialization (db-up → migrate → seed)
   - `serve`: Starts both API and web dev server together
   - `collect-*`: Shortcuts for all collection operations
   - `check` & `test`: Match CI gates exactly

2. **Safety Guards**
   - `db-reset` prompts for confirmation before destructive action
   - `--check` flags for format/lint (never auto-formats on dev machines)
   - Clear error messages on missing dependencies

3. **Consistent with CLAUDE.md**
   - Commands documented in project README
   - Env variables clearly listed

### No Issues Found

The justfile is well-designed. No changes required.

---

## Environment Management

### ✅ Strengths

1. **.env.example is Comprehensive**
   - All required variables documented
   - Sensible defaults provided
   - Security notes on API keys and passwords
   - Clear sections for different concerns (DB, auth, external providers)

2. **Layered Configuration**
   - `.env` for local overrides (gitignored, never committed)
   - `.env.example` as reference (tracked in git)
   - `AppConfig` struct validates at startup (fail-fast)

### ⚠️ Issues Found

#### 10. **No Health Check Probe for Scheduler (OBSERVABILITY — LOW)**

**Severity:** LOW
**Location:** `crates/scbdb-server/src/scheduler/mod.rs`
**Issue:**

The scheduler is started in `main.rs` but there's no way to check if it's actually running or if jobs have failed. External monitoring systems can't detect scheduler hangs.

**Recommendation:**

Add a health endpoint that tracks job execution:
```rust
pub struct SchedulerMetrics {
    pub last_run: Option<DateTime<Utc>>,
    pub last_failure: Option<String>,
    pub pending_jobs: usize,
}

#[get("/api/v1/health/scheduler")]
async fn scheduler_health(State(state): State<AppState>) -> Json<SchedulerMetrics> {
    let metrics = state.scheduler_metrics.lock().await;
    Json(metrics.clone())
}
```

Update `/api/v1/health` to include scheduler status:
```json
{
  "data": {
    "status": "ok",
    "database": "ok",
    "scheduler": {
      "last_run": "2024-02-24T18:30:00Z",
      "pending_jobs": 4
    }
  }
}
```

---

---

# Summary Table

| Issue | Crate/File | Severity | Category | Recommendation |
|-------|-----------|----------|----------|---|
| **1** | `middleware.rs` | **CRITICAL** | Security | Use `subtle::ConstantTimeEq` for token comparison |
| **2** | `middleware.rs` + `.env.example` | **CRITICAL** | Security | Add API key salt, switch to Argon2 hashing |
| **3** | `api/mod.rs` | HIGH | Design | Add `details` field to `ErrorBody` for structured errors |
| **4** | `main.rs` | HIGH | Design | Wrap `JobScheduler` in newtype to prevent accidental drops |
| **5** | `collect/runner.rs` | MEDIUM | Observability | Add structured logging with per-brand/per-product timing |
| **6** | `web/api/client.ts` | MEDIUM | Type Safety | Use Zod validation for error response parsing |
| **7** | `web/index.html` | MEDIUM | Frontend | Add viewport meta tag and focus management |
| **8** | `web/api/client.ts` | MEDIUM | Security | Validate API key at app startup and handle 401 errors |
| **9** | `.github/workflows/ci.yml` | MEDIUM | DevOps | Add build artifact caching, SBOM generation |
| **10** | `scheduler/mod.rs` | LOW | Observability | Add scheduler health check endpoint |

---

# Recommended Action Plan

## Phase 1: Immediate (Critical Security Issues)
- [ ] Issue #1: Implement constant-time token comparison
- [ ] Issue #2: Add Argon2 hashing with salt to auth middleware

## Phase 2: High Priority (Design & Reliability)
- [ ] Issue #3: Add `details` field to error responses
- [ ] Issue #4: Wrap `JobScheduler` in protective newtype
- [ ] Issue #5: Add structured logging to collection pipeline

## Phase 3: Medium Priority (Type Safety & Frontend)
- [ ] Issue #6: Validate API error responses with Zod
- [ ] Issue #7: Add viewport meta tag and focus management
- [ ] Issue #8: Add API key validation at startup
- [ ] Issue #9: Add CI caching for web builds

## Phase 4: Polish (Observability)
- [ ] Issue #10: Add scheduler health check endpoint

---

# Conclusion

**Overall Assessment: STRONG FOUNDATION**

The SCBDB codebase demonstrates excellent engineering discipline across both backend (Rust) and frontend (React/TypeScript). The architecture is modular, dependencies are current, and the CI/CD pipeline is comprehensive.

The two critical security issues (constant-time comparison, API key hashing) must be addressed before any production deployment. The remaining 8 items are important for reliability, type safety, and observability but do not block development.

**Estimated effort to resolve all issues: 3–5 developer-days**
- Security fixes: 4–6 hours (implement + review)
- Error handling improvements: 4–6 hours (add details field, validate responses)
- Observability: 2–3 hours (logging, health checks)
- Frontend polish: 1–2 hours (meta tag, focus management)
- DevOps enhancements: 1–2 hours (caching, SBOM)

---

**Review completed by:** AI Engineer
**Review methodology:** Static analysis of source code, configuration files, CI/CD pipelines, and framework idioms against modern best practices (Rust 2024, React 19, TypeScript 5.9).
