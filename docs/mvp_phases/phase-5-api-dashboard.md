# Phase 5: API and Dashboard

## Document Metadata

- Version: 1.1
- Status: Completed
- Last Updated (EST): 21:30:00 | 02/20/2026 EST

## Objective

Deliver production-ready API surfaces and a web dashboard that exposes all four data domains (products, pricing, regulatory, sentiment) through a unified React interface.

## Outcomes Delivered

- Axum REST API with four data domains under `/api/v1`
- Bearer-token auth, request-ID tracing, and rate-limiting middleware
- React dashboard with Products, Pricing, Regulatory, and Sentiment tabs
- TanStack Query data layer (hooks, fetch functions, TypeScript types)
- 7 passing web tests (smoke, client, dashboard-page with 4 scenarios)
- 300 passing Rust tests across all workspace crates

## API Endpoints Shipped

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/v1/health` | Public liveness check |
| `GET` | `/api/v1/products` | Product catalog with optional filters |
| `GET` | `/api/v1/pricing/summary` | Latest price per brand/variant |
| `GET` | `/api/v1/pricing/snapshots` | Full price history, filterable |
| `GET` | `/api/v1/bills` | Legislative bills with optional filters |
| `GET` | `/api/v1/bills/{bill_id}/events` | Timeline events for one bill |
| `GET` | `/api/v1/sentiment/summary` | Most recent score per active brand |
| `GET` | `/api/v1/sentiment/snapshots` | Recent snapshot feed, `?limit=N` |

All authenticated endpoints require `Authorization: Bearer <api_key>`. Auth is disabled when `SCBDB_API_KEYS` is unset (development mode).

## Server Architecture

The server crate (`crates/scbdb-server/`) uses a modular layout:

```
src/
  main.rs          — entrypoint, AppState, shutdown
  middleware.rs     — RequestId, BearerAuth, RateLimit
  api/
    mod.rs         — build_app, router assembly, shared types, helpers
    products.rs    — list_products handler
    pricing.rs     — list_pricing_snapshots, list_pricing_summary handlers
    bills.rs       — list_bills, list_bill_events handlers
    sentiment.rs   — list_sentiment_summary, list_sentiment_snapshots handlers
```

Each handler follows the same pattern: extract `State<AppState>` + `Extension<RequestId>`, call a DB query function from `scbdb-db`, map rows to API types, return `Json<ApiResponse<T>>`.

## Frontend Architecture

The web project (`web/`) uses a component-per-panel layout:

```
src/
  components/
    dashboard-page.tsx        — orchestrator: hooks, tabStats, panel routing
    dashboard-utils.tsx       — formatDate, formatMoney, formatScore, scoreClass, scorePct, LoadingState, ErrorState
    products-panel.tsx        — product card grid
    pricing-panel.tsx         — pricing summary cards + snapshot mini-table
    regulatory-panel.tsx      — bill list with status badges
    sentiment-panel.tsx       — score badge cards with meter bar + recent runs table
  hooks/
    use-dashboard-data.ts     — useProducts, usePricingSummary, usePricingSnapshots, useBills, useSentimentSummary, useSentimentSnapshots
  lib/api/
    client.ts                 — apiGet helper, base URL validation
    dashboard.ts              — fetch functions for all six queries
  types/
    api.ts                    — TypeScript types mirroring server response shapes
  styles.css                  — CSS variables, layout, card components, sentiment badge/meter
```

## Sentiment Tab Detail

See [docs/SENTIMENT_DASHBOARD.md](../SENTIMENT_DASHBOARD.md) for the full design and component reference.

## Files Changed

| Layer | Files |
|-------|-------|
| DB | `crates/scbdb-db/src/api_queries.rs`, `crates/scbdb-db/src/lib.rs` |
| API | `crates/scbdb-server/src/api/` (all five files) |
| Web types | `web/src/types/api.ts` |
| Web fetch | `web/src/lib/api/dashboard.ts` |
| Web hooks | `web/src/hooks/use-dashboard-data.ts` |
| Web UI | `web/src/components/` (all panel files) |
| Web styles | `web/src/styles.css` |
| Web config | `web/vite.config.ts` |
| Tests | `web/src/components/dashboard-page.test.tsx`, `web/src/lib/api/client.test.ts` |
