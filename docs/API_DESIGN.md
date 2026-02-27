# API Design

## Document Metadata

- Version: 1.2
- Status: Active
- Last Updated (EST): 20:22:00 | 02/26/2026 EST

## Purpose

Defines the current REST contract for `scbdb-server` and explicitly marks planned-but-not-implemented items.

## Contract Source of Truth

- Router surface: `crates/scbdb-server/src/api/mod.rs`
- Handler/query behavior: `crates/scbdb-server/src/api/*.rs`

## Base URL and Versioning

- Base path: `/api/v1`
- Content type: `application/json`
- Versioning strategy: path-based (`/api/v1`)
- Endpoint paths below are listed relative to `/api/v1`.

## Authentication

- Scheme (when enabled): `Authorization: Bearer <api_key>`
- Public endpoint: `GET /health`
- Protected endpoints: all other `/api/v1/*` routes
- Environment behavior: in development, auth is disabled if `SCBDB_API_KEYS` is unset/empty

## Response Envelope

### Success

```json
{
  "data": {},
  "meta": {
    "request_id": "req_123",
    "timestamp": "2026-02-26T00:00:00Z"
  }
}
```

### Error

```json
{
  "error": {
    "code": "validation_error",
    "message": "Invalid tier value"
  },
  "meta": {
    "request_id": "req_123",
    "timestamp": "2026-02-26T00:00:00Z"
  }
}
```

Note: `error.details` is not currently part of the implemented error schema.

## Pagination

- Shared limit behavior (where supported): `limit` defaults to `50`, clamped to `1..=200`.
- Cursor pagination is currently implemented on specific endpoints only:
  - `GET /brands/{slug}/signals` (query: `cursor`, `limit`)
  - `GET /locations/pins` (query: `cursor`, `limit`, optional `brand_slug`)
- `next_cursor` is returned inside `data`, not in `meta`.

## Path Parameters

- `{slug}`: brand slug string (e.g. `cann`, `jones-soda`)
- `{bill_id}`: UUID public bill id
- Internal integer primary keys are not used as path params.

## Implemented Endpoints

### System

- `GET /health`
  - Public health check
  - Returns `200` when DB is healthy, `503` when DB is unavailable

### Brands

- `GET /brands`
- `POST /brands`
- `GET /brands/{slug}`
- `PATCH /brands/{slug}`
- `DELETE /brands/{slug}` (soft deactivate)
- `GET /brands/{slug}/signals`
  - Query: `type`, `cursor`, `limit`
- `GET /brands/{slug}/funding`
- `GET /brands/{slug}/lab-tests`
- `GET /brands/{slug}/legal`
- `GET /brands/{slug}/sponsorships`
- `GET /brands/{slug}/distributors`
- `GET /brands/{slug}/competitors`
- `GET /brands/{slug}/media`
- `PUT /brands/{slug}/profile`
- `PUT /brands/{slug}/social`
- `PUT /brands/{slug}/domains`

### Products

- `GET /products`
  - Query: `brand_slug`, `relationship`, `tier`, `limit`

### Pricing

- `GET /pricing/snapshots`
  - Query: `brand_slug`, `from`, `to`, `limit`
- `GET /pricing/summary`

### Regulatory

- `GET /bills`
  - Query: `jurisdiction`, `limit`
- `GET /bills/{bill_id}/events`
- `GET /bills/{bill_id}/texts`

### Sentiment

- `GET /sentiment/summary`
- `GET /sentiment/snapshots`
  - Query: `limit`

### Locations

- `GET /locations/summary`
- `GET /locations/by-state`
- `GET /locations/pins`
  - Query: `cursor`, `limit`, `brand_slug`

## Planned / Not Implemented

### API Routes

- `GET /collection-runs` is documented in earlier plans but is not currently routed.
- `POST /collection-runs/products` is not implemented.
- `POST /collection-runs/pricing` is not implemented.
- `POST /collection-runs/regs` is not implemented.
- `GET /products/{product_id}` is not implemented.
- `GET /products/{product_id}/variants` is not implemented.
- `GET /bills/{bill_id}` is not implemented.

### OpenAPI Status

- Planned: expose generated spec at `/api/v1/openapi.json`
- Current state: not implemented

### Idempotency Status

- Planned: explicit `Idempotency-Key` handling for replay-safe writes
- Current state: no `Idempotency-Key` middleware/contract enforcement implemented
- Note: `PUT /brands/{slug}/profile|social|domains` are semantically idempotent by HTTP method, but not keyed by idempotency token

### Report Command Status

- `scbdb-cli report` (top-level): planned, currently stubbed/not implemented
- `scbdb-cli regs report`: implemented
- `scbdb-cli sentiment report`: implemented

## HTTP Status Codes

- `200 OK`: read/update/deactivate success
- `201 Created`: brand create success (`POST /brands`)
- `400 Bad Request`: invalid params/body
- `401 Unauthorized`: missing/invalid bearer token (when auth enabled)
- `404 Not Found`: missing resource
- `409 Conflict`: uniqueness/state conflict
- `429 Too Many Requests`: rate limit exceeded
- `500 Internal Server Error`: unhandled failure
- `503 Service Unavailable`: degraded health check (DB unavailable)
