# Development

## Module Structure

All code must be small, focused modules. No monolithic files. Every module has a single responsibility.

### Rules

- One concern per file. If a module does two things, split it.
- Target ~200 lines per file. Hard max 300. If it's over 300, it must be split — no exceptions.
- `#[cfg(test)]` blocks and co-located test modules (`*.test.ts`) do not count against the limit. Only production code counts.
- Public API of each module is explicit — re-export from `mod.rs` / `lib.rs`, keep internals private.
- Traits define boundaries between layers. Concrete types implement traits; consumers depend on traits.

### Rust Crate Internals

Each library crate follows the same internal layout:

```
crates/scbdb-scraper/src/
  lib.rs              # public re-exports only
  client.rs           # HTTP client construction
  pagination.rs       # page iteration logic
  normalize.rs        # Shopify JSON → internal model mapping
  rate_limit.rs       # throttling / backoff
  error.rs            # crate-specific error types
```

`lib.rs` is a manifest, not an implementation file:

```rust
// crates/scbdb-scraper/src/lib.rs
mod client;
mod error;
mod normalize;
mod pagination;
mod rate_limit;

pub use client::ShopifyClient;
pub use error::ScraperError;
pub use normalize::normalize_product;
```

### Frontend Modules

Components, hooks, and utilities each get their own file. No barrel files that re-export everything — import from the specific module.

```
web/src/
  components/
    products/
      ProductCard.tsx
      ProductCard.test.tsx
      ProductTable.tsx
      ProductTable.test.tsx
    legislation/
      BillSummary.tsx
      BillSummary.test.tsx
  hooks/
    useProducts.ts
    useProducts.test.ts
    useBills.ts
    useBills.test.ts
  lib/
    api/
      client.ts         # shared fetch wrapper
      products.ts       # product endpoints
      legislation.ts    # legislation endpoints
      sentiment.ts      # sentiment endpoints
    format/
      price.ts
      price.test.ts
      date.ts
      date.test.ts
  types/
    product.ts
    bill.ts
    sentiment.ts
```

### Separation of Concerns

| Layer | Rust crate | Responsibility |
|---|---|---|
| Domain models | `scbdb-core` | Types, traits, validation — zero I/O |
| Persistence | `scbdb-db` | Database queries, migrations — no business logic |
| Collection | `scbdb-scraper`, `scbdb-legiscan` | External API interaction — no persistence, returns domain types |
| Analysis | `scbdb-sentiment` | Computation over domain types — no I/O |
| Presentation | `scbdb-server` | HTTP routing, serialization — delegates to other crates |
| Orchestration | `scbdb-cli` | Wires layers together, handles user input — no business logic |

No layer reaches into another's internals. Data flows through the public trait boundaries defined in `scbdb-core`.

## Mobile-First Web Development

Every UI component is designed for small screens first, then enhanced for larger viewports. No desktop-first CSS.

### Principles

- **Start at 320px.** If it doesn't work on a small phone, it doesn't ship.
- **Enhance upward.** Base styles are mobile. Breakpoints add complexity, never remove it.
- **Touch targets first.** Minimum 44x44px for all interactive elements. No hover-only interactions.
- **Content priority.** Decide what matters on a 5-inch screen before thinking about sidebar layouts.

### Tailwind CSS 4+ Breakpoints

Write base styles for mobile. Use `sm:`, `md:`, `lg:`, `xl:` to layer on tablet/desktop overrides.

```tsx
// correct — mobile-first
<div className="flex flex-col gap-4 md:flex-row md:gap-6 lg:gap-8">
  <main className="w-full md:w-2/3">...</main>
  <aside className="w-full md:w-1/3">...</aside>
</div>

// wrong — desktop-first (undoing at smaller sizes)
<div className="flex flex-row gap-8 sm:flex-col sm:gap-4">
```

Standard breakpoints:

| Prefix | Min width | Target |
|---|---|---|
| *(none)* | 0px | Phones (default) |
| `sm:` | 640px | Large phones / small tablets |
| `md:` | 768px | Tablets |
| `lg:` | 1024px | Laptops |
| `xl:` | 1280px | Desktops |

### Layout Patterns

#### Stacking → Side-by-side

The most common pattern. Content stacks vertically on mobile, flows horizontally on larger screens.

```tsx
// components/products/ProductGrid.tsx
export function ProductGrid({ products }: Props) {
  return (
    <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
      {products.map((p) => (
        <ProductCard key={p.id} product={p} />
      ))}
    </div>
  );
}
```

#### Responsive Tables

Data tables collapse to card layouts on mobile. Never force horizontal scroll on small screens for primary data.

```
web/src/
  components/
    data-display/
      ResponsiveTable.tsx       # table on md+, cards on mobile
      ResponsiveTable.test.tsx
      DataCard.tsx              # single-record card view
      DataCard.test.tsx
```

```tsx
// components/data-display/ResponsiveTable.tsx
export function ResponsiveTable<T>({ data, columns, renderCard }: Props<T>) {
  return (
    <>
      {/* card layout — mobile */}
      <div className="flex flex-col gap-3 md:hidden">
        {data.map(renderCard)}
      </div>
      {/* table — tablet+ */}
      <table className="hidden md:table w-full">
        ...
      </table>
    </>
  );
}
```

#### Navigation

Bottom nav on mobile, sidebar on desktop. One component per pattern, composed in the layout.

```
web/src/
  components/
    layout/
      AppShell.tsx            # composes nav + content area
      AppShell.test.tsx
      BottomNav.tsx           # mobile: fixed bottom bar
      BottomNav.test.tsx
      Sidebar.tsx             # desktop: collapsible side nav
      Sidebar.test.tsx
```

```tsx
// components/layout/AppShell.tsx
export function AppShell({ children }: Props) {
  return (
    <div className="min-h-screen pb-16 md:pb-0 md:pl-64">
      <Sidebar className="hidden md:flex" />
      <main className="p-4 md:p-6 lg:p-8">{children}</main>
      <BottomNav className="fixed bottom-0 left-0 right-0 md:hidden" />
    </div>
  );
}
```

### shadcn/ui Responsive Rules

- Use `Sheet` (slide-over) for mobile modals, `Dialog` for desktop. Switch with a `useMediaQuery` hook.
- `DropdownMenu` on desktop, `Drawer` on mobile for action menus.
- `Popover` panels should be full-width on mobile (`w-screen sm:w-auto`).
- Keep each shadcn wrapper in its own file — don't build a mega-component.

```
web/src/
  hooks/
    useMediaQuery.ts
    useMediaQuery.test.ts
    useIsMobile.ts           # thin wrapper: useMediaQuery("(max-width: 767px)")
    useIsMobile.test.ts
```

### Touch & Interaction

- Minimum touch target: `min-h-11 min-w-11` (44px at default scale).
- Swipe gestures for navigation (tab switching, dismissing sheets) — use a dedicated hook per gesture.
- No hover-dependent UI. `:hover` is progressive enhancement only — the feature must work without it.
- Use `active:` states for touch feedback (`active:scale-95`, `active:bg-muted`).

### Performance on Mobile

- Lazy-load below-the-fold content with `React.lazy` + `Suspense`.
- Virtualize long lists (product catalogs, bill lists) — don't render 500 DOM nodes on a phone.
- Images: use `srcSet` / `sizes` for responsive images. Serve WebP with AVIF fallback.
- Keep the initial JS bundle under 200KB gzipped. Code-split by route.

```
web/src/
  components/
    common/
      LazyImage.tsx           # responsive srcSet + loading="lazy"
      LazyImage.test.tsx
      VirtualList.tsx          # windowed rendering for long lists
      VirtualList.test.tsx
```

### Testing Responsive Behavior

- Test mobile and desktop variants separately in component tests using `useMediaQuery` mocks.
- Viewport-specific assertions: verify that mobile-only elements render and desktop-only elements don't (and vice versa).

```tsx
// components/layout/AppShell.test.tsx
describe("AppShell", () => {
  it("renders bottom nav on mobile", () => {
    mockMediaQuery("(max-width: 767px)", true);
    render(<AppShell>content</AppShell>);
    expect(screen.getByRole("navigation", { name: /bottom/i })).toBeVisible();
  });

  it("renders sidebar on desktop", () => {
    mockMediaQuery("(max-width: 767px)", false);
    render(<AppShell>content</AppShell>);
    expect(screen.getByRole("navigation", { name: /sidebar/i })).toBeVisible();
  });
});
```

## TDD Workflow

Strict red-green-refactor for every change. No production code is written without a failing test first.

### The Cycle

1. **Red** — Write a test that describes the expected behavior. Run it. It must fail.
2. **Green** — Write the minimum production code to make the test pass. Nothing more.
3. **Refactor** — Clean up the implementation and the test. All tests must still pass.

### Rules

- Never write production code without a failing test.
- Never write more test code than is needed to produce a failure (a compile error counts as a failure).
- Never write more production code than is needed to pass the currently failing test.
- Refactor only when all tests are green.
- Every commit must leave the test suite green.

## Rust Testing

### Unit Tests

Co-located in each module using `#[cfg(test)]` blocks. Test the module's public interface.

```rust
// crates/scbdb-core/src/product.rs

pub fn normalize_price(cents: i64) -> f64 {
    cents as f64 / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_price_converts_cents_to_dollars() {
        assert_eq!(normalize_price(1999), 19.99);
    }
}
```

### Async Tests

Any test touching sqlx, reqwest, or axum handlers uses `#[tokio::test]` for the async runtime.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fetches_products_from_shopify() {
        let mock_server = wiremock::MockServer::start().await;
        // ...
    }
}
```

### HTTP Mocking

**wiremock** provides a mock HTTP server for testing the Shopify scraper and LegiScan client without hitting real APIs. Each test gets its own server instance — no shared state, no port conflicts.

```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn scraper_handles_empty_product_list() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/products.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"products": []})))
        .mount(&mock)
        .await;

    let client = ShopifyClient::new(&mock.uri());
    let products = client.fetch_all().await.unwrap();
    assert!(products.is_empty());
}
```

### Database Tests

**`#[sqlx::test]`** gives each test function its own migrated PostgreSQL database. Migrations run automatically, the database is dropped after the test. No manual setup, no cleanup, no test ordering issues.

```rust
#[sqlx::test]
async fn inserts_and_retrieves_product(pool: PgPool) {
    let product = Product { name: "Test".into(), /* ... */ };
    insert_product(&pool, &product).await.unwrap();

    let found = get_product_by_name(&pool, "Test").await.unwrap();
    assert_eq!(found.name, "Test");
}
```

Requires `DATABASE_URL` set in `.env` or environment. sqlx creates temporary databases from this connection.

### Integration Tests

Per-crate `tests/` directory for tests that cross module boundaries or hit external interfaces (database, HTTP).

```
crates/
  scbdb-scraper/
    src/
    tests/
      shopify_pagination.rs
      shopify_normalization.rs
  scbdb-db/
    src/
    tests/
      migrations.rs
      product_queries.rs
```

### Test Utilities

Shared test fixtures and helpers live in `scbdb-core` behind a `test-utils` feature flag so they're available to all crates during testing but excluded from release builds.

```toml
# crates/scbdb-core/Cargo.toml
[features]
test-utils = []
```

```toml
# crates/scbdb-scraper/Cargo.toml
[dev-dependencies]
scbdb-core = { path = "../scbdb-core", features = ["test-utils"] }
wiremock = "0.6"
```

## Frontend Testing

### Unit Tests

**Vitest** for all component and utility tests. Co-located test files using `*.test.ts` / `*.test.tsx` alongside source files.

### Component Tests

**React Testing Library** for component behavior testing. Test user interactions, not implementation details.

### Test Structure

```
web/src/
  components/
    ProductCard.tsx
    ProductCard.test.tsx
  lib/
    format.ts
    format.test.ts
```

## Running Tests

```sh
# rust — full workspace
cargo test --workspace

# rust — single crate
cargo test -p scbdb-scraper

# rust — single test
cargo test -p scbdb-scraper -- test_name

# frontend
cd web && npm test

# everything (via justfile)
just test
```

## Linting & Formatting

### Rust

- `cargo fmt --all` — format all crates (rustfmt, zero config)
- `cargo clippy --workspace -- -D warnings` — lint with all warnings as errors

### Frontend

- `npx eslint .` — lint TypeScript/React (ESLint 9+ flat config with `eslint-plugin-react-hooks` and `eslint-plugin-jsx-a11y`)
- `npx prettier --check .` — format check (TypeScript, TSX, JSON, Markdown, CSS)
- `npx prettier --write .` — auto-format all files

### Type Checking

Vite intentionally skips type checking for build speed. TypeScript type errors will ship silently unless `tsc` is run separately.

- `cargo check --workspace` — fast Rust type checking without full compilation
- `cd web && npx tsc --noEmit` — TypeScript type checking (must pass in CI)

### Pre-commit (lefthook)

**lefthook** manages git hooks. Single Go binary, no Node or Python runtime required. Runs Rust and frontend checks in parallel.

Install: `brew install lefthook` (macOS) or `cargo install lefthook` or download the binary directly.

```yaml
# lefthook.yml
pre-commit:
  parallel: true
  commands:
    rust-fmt:
      glob: "*.rs"
      run: cargo fmt --all -- --check
    rust-clippy:
      glob: "*.rs"
      run: cargo clippy --workspace -- -D warnings
    ts-lint:
      glob: "*.{ts,tsx}"
      run: cd web && npx eslint .
    ts-typecheck:
      glob: "*.{ts,tsx}"
      run: cd web && npx tsc --noEmit
    prettier:
      glob: "*.{ts,tsx,json,md,css}"
      run: npx prettier --check .
```

After cloning the repo, run `lefthook install` to set up the hooks. This is a one-time step.

The justfile also provides a manual command for running all checks without committing:

```sh
just check   # fmt + clippy + tsc --noEmit + test + eslint + prettier
```

## Migrations

SQL migration files live in `migrations/` at the repo root. Managed by sqlx's built-in migration runner.

### Commands

```sh
# create a new migration
sqlx migrate add <name>         # creates migrations/<timestamp>_<name>.sql

# apply pending migrations
sqlx migrate run                # or: just migrate

# revert last migration
sqlx migrate revert

# check migration status
sqlx migrate info
```

### Rules

- Migrations are **append-only**. Never edit a migration that has been applied to any environment.
- Each migration file is a single `.sql` file with plain SQL. No Rust code in migrations.
- Destructive changes (dropping columns/tables) require a two-step migration: first deprecate, then remove in a later migration.
- `DATABASE_URL` must be set in `.env` for sqlx to connect. Format: `postgres://user:pass@localhost:5432/scbdb`

## Error Handling

### Library Crates — thiserror

Every library crate defines its own error enum in `error.rs` using **thiserror**. Errors are specific, matchable, and carry context.

```rust
// crates/scbdb-scraper/src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScraperError {
    #[error("HTTP request failed for {url}: {source}")]
    Http {
        url: String,
        source: reqwest::Error,
    },

    #[error("failed to parse product JSON: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("rate limited by {domain}, retry after {retry_after_secs}s")]
    RateLimited {
        domain: String,
        retry_after_secs: u64,
    },
}
```

### Binary Crates — anyhow

**`scbdb-cli`** and **`scbdb-server`** use **anyhow** for top-level error propagation. They convert library errors into user-facing messages — they never expose internal error types to end users.

```rust
// crates/scbdb-cli/src/main.rs
use anyhow::{Context, Result};

fn main() -> Result<()> {
    let config = load_config()
        .context("failed to load configuration")?;
    // ...
}
```

### Rules

- **Library crates use thiserror. Binary crates use anyhow.** No exceptions.
- Every error variant includes enough context to diagnose the problem without a debugger.
- Use `.context()` / `.with_context()` when propagating errors to add what-was-happening information.
- Never use `.unwrap()` or `.expect()` in production code. Tests may use them freely.
- Never swallow errors with `let _ = ...`. Either handle them or propagate them.

### Frontend — Error Boundaries

React Error Boundaries catch component crashes and render fallback UI instead of a white screen. One boundary wraps the app root, additional boundaries wrap major page sections.

```tsx
// components/common/ErrorBoundary.tsx
import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props { children: ReactNode; fallback: ReactNode; }
interface State { hasError: boolean; }

export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false };

  static getDerivedStateFromError(): State {
    return { hasError: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("ErrorBoundary caught:", error, info);
  }

  render() {
    return this.state.hasError ? this.props.fallback : this.props.children;
  }
}
```

## Logging & Tracing

### Rust — tracing

All Rust code uses **tracing** for structured, span-based logging. Never use `println!` or `eprintln!` for operational output — those are for user-facing CLI messages only.

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(pool), fields(brand = %brand.name))]
pub async fn scrape_brand(pool: &PgPool, brand: &Brand) -> Result<ScrapeResult> {
    info!("starting scrape");
    let products = fetch_products(&brand.shopify_url).await?;
    info!(count = products.len(), "fetched products");
    // ...
}
```

### Log Levels

| Level | Use for |
|---|---|
| `error!` | Failures that need investigation — failed scrapes, DB errors, unexpected state |
| `warn!` | Recoverable issues — rate limiting, retries, missing optional data |
| `info!` | Operational milestones — scrape started/completed, server listening, migration applied |
| `debug!` | Implementation details — SQL queries, HTTP request/response bodies, parsed data |
| `trace!` | Per-item iteration — individual product processing, field mapping |

### Configuration

Log levels are controlled via the `RUST_LOG` environment variable using **tracing-subscriber**.

```sh
RUST_LOG=info                          # default for production
RUST_LOG=scbdb_scraper=debug           # debug a specific crate
RUST_LOG=debug,hyper=info,sqlx=warn    # verbose, but quiet noisy deps
```

### Axum Integration

The HTTP server uses **tower-http**'s `TraceLayer` for automatic request/response logging.

```rust
use axum::Router;
use tower_http::trace::TraceLayer;

let app = Router::new()
    .nest("/api", api_routes)
    .layer(TraceLayer::new_for_http());
```

This logs every request with method, path, status code, and latency — no per-handler logging boilerplate needed.

### Rules

- Use `#[instrument]` on async functions that represent meaningful operations. Skip large arguments (`skip(pool)`, `skip(body)`).
- Add structured fields, not string interpolation: `info!(count = products.len(), "fetched products")` not `info!("fetched {} products", products.len())`.
- Never log secrets, tokens, or full database connection strings.

## Commit Discipline

- Each commit represents one red-green-refactor cycle (or a refactor-only step).
- The test suite is green on every commit.
- Commit messages describe the behavior added, not the code changed.
