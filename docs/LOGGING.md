# Logging & Error Handling

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
