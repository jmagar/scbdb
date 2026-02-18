# Development

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
