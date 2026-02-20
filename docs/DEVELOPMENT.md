# Development

## Document Metadata

- Version: 1.1
- Status: Active
- Last Updated (EST): 18:55:35 | 02/18/2026 EST

## Justfile Commands

The `just` command interface is the standard task runner contract for local development and CI parity. Keep this section synchronized with the implemented `justfile`.

| Command | Purpose |
|---|---|
| `just dev` | Start local development stack (backend/frontend helpers as configured) |
| `just build` | Build workspace artifacts |
| `just test` | Run all tests (Rust + frontend) |
| `just check` | Run lint, type-check, and formatting checks |
| `just migrate` | Apply database migrations |
| `just migrate-status` | Show migration state |
| `just db-up` | Start PostgreSQL services via Docker Compose |
| `just db-down` | Stop PostgreSQL services |
| `just format` | Apply formatters (`cargo fmt`, frontend formatter) |
| `just clean` | Remove local build artifacts |

Use `just --list` to verify the canonical command list once the `justfile` is present.

Current status: the command table above is implemented in the repository `justfile`.

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

```text
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

```text
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

```text
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

```text
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

```text
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

```text
web/src/
  components/
    common/
      LazyImage.tsx           # responsive srcSet + loading="lazy"
      LazyImage.test.tsx
      VirtualList.tsx          # windowed rendering for long lists
      VirtualList.test.tsx
```

### Testing Responsive Behavior

See [TESTING.md](TESTING.md) for responsive testing patterns with `useMediaQuery` mocks and viewport-specific assertions.

## Testing

See [TESTING.md](TESTING.md) for the full testing guide — TDD workflow, Rust testing (unit, async, HTTP mocking, database tests, integration), frontend testing (Vitest, React Testing Library), and test commands.

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
    check-file-size:
      run: scripts/check-file-size.sh
      fail_text: "Split large files into focused modules before committing."
    cargo-fmt:
      glob: "**/*.rs"
      run: cargo fmt --all -- --check
      fail_text: "Run 'just format' to fix Rust formatting."
    web-format:
      glob: "web/**/*.{ts,tsx,css}"
      run: pnpm --dir web format:check
      fail_text: "Run 'just format' to fix web formatting."

pre-push:
  parallel: true
  commands:
    cargo-clippy:
      glob: "**/*.rs"
      run: cargo clippy --workspace -- -D warnings
      fail_text: "Run 'just check' to see clippy details."
    cargo-test:
      glob: "**/*.rs"
      run: cargo test --workspace
      fail_text: "All tests must pass before pushing."
    web-typecheck:
      glob: "web/**/*.{ts,tsx}"
      run: pnpm --dir web typecheck
      fail_text: "Fix TypeScript errors before pushing."
```

After cloning the repo, run `lefthook install` to set up the hooks. This is a one-time step.

The justfile also provides a manual command for running all checks without committing:

```sh
just check   # fmt + clippy + tsc --noEmit + test + eslint + prettier
```

## Local Development Environment

### Docker Compose

PostgreSQL runs via Docker Compose. No local database installation required.

```sh
# start PostgreSQL
docker compose up -d

# stop services
docker compose down

# stop and remove volumes (full reset)
docker compose down -v
```

`DATABASE_URL` in `.env` should point to the Docker Compose PostgreSQL instance: `postgres://scbdb:scbdb@localhost:15432/scbdb`

### Environment Variables

Copy `.env.example` to `.env` and fill in required values:

```sh
SCBDB_ENV=development
DATABASE_URL=postgres://scbdb:scbdb@localhost:15432/scbdb
SCBDB_API_KEY_HASH_SALT=<change-me>
SCBDB_BIND_ADDR=0.0.0.0:3000
SCBDB_LOG_LEVEL=info
SCBDB_BRANDS_PATH=./config/brands.yaml
LEGISCAN_API_KEY=<your-key>
```

`dotenvy` loads `.env` automatically in both `scbdb-cli` and `scbdb-server`.
See `CONFIG_LOADING.md` for full precedence and validation rules.

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
- `DATABASE_URL` must be set in `.env` for sqlx to connect. Format: `postgres://user:pass@localhost:15432/scbdb`

## Error Handling & Logging

See [LOGGING.md](LOGGING.md) for the full guide — thiserror/anyhow split, error handling rules, React Error Boundaries, tracing setup, log levels, and Axum integration.

## Commit Discipline

- Each commit represents one red-green-refactor cycle (or a refactor-only step).
- The test suite is green on every commit.
- Commit messages describe the behavior added, not the code changed.
