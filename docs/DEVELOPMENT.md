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

### Pre-commit

All checks must pass before committing. The justfile provides a single command:

```sh
just check   # fmt + clippy + tsc --noEmit + test + eslint + prettier
```

## Commit Discipline

- Each commit represents one red-green-refactor cycle (or a refactor-only step).
- The test suite is green on every commit.
- Commit messages describe the behavior added, not the code changed.
