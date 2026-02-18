# Testing

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

### Testing Responsive Behavior

Test mobile and desktop variants separately in component tests using `useMediaQuery` mocks. Viewport-specific assertions verify that mobile-only elements render and desktop-only elements don't (and vice versa).

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
