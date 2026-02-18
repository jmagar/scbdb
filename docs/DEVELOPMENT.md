# Development

## Module Structure

All code must be small, focused modules. No monolithic files. Every module has a single responsibility.

### Rules

- One concern per file. If a module does two things, split it.
- No file should exceed ~200 lines. If it does, it needs decomposition.
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

- `cargo fmt --all` — format all crates
- `cargo clippy --workspace -- -D warnings` — lint with warnings as errors

### Frontend

- `npx eslint .` — lint TypeScript/React
- `npx prettier --check .` — format check

### Pre-commit

All checks must pass before committing. The justfile provides a single command:

```sh
just check   # fmt + clippy + test + lint + prettier
```

## Commit Discipline

- Each commit represents one red-green-refactor cycle (or a refactor-only step).
- The test suite is green on every commit.
- Commit messages describe the behavior added, not the code changed.
