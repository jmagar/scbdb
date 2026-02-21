# API Design

## Document Metadata

- Version: 1.2
- Status: Active
- Last Updated (EST): 21:30:00 | 02/20/2026 EST

## Purpose

Defines REST API conventions and endpoint surface for `scbdb-server`.

## Base URL and Versioning

- Base path: `/api/v1`
- Content type: `application/json`
- Versioning strategy: path-based (`/api/v1`)
- Endpoint paths below are listed relative to `/api/v1`.

## Authentication

- Scheme: `Authorization: Bearer <api_key>`
- All endpoints require API keys except `/health`.

## Standard Response Envelope

### Success

```json
{
  "data": {},
  "meta": {
    "request_id": "req_123",
    "timestamp": "2026-02-18T00:00:00Z"
  }
}
```

### Error

```json
{
  "error": {
    "code": "validation_error",
    "message": "Invalid tier value",
    "details": {
      "field": "tier"
    }
  },
  "meta": {
    "request_id": "req_123",
    "timestamp": "2026-02-18T00:00:00Z"
  }
}
```

## Pagination

- Cursor-based pagination for large lists.
- Query params:
  - `limit` (default `50`, max `200`)
  - `cursor` (opaque token)
- Response `meta` includes `next_cursor` when more data exists.

## Endpoints

### System

- `GET /health`
  - Public status check.

### Path Parameter Convention

All `{*_id}` path parameters accept the **public UUID** (`public_id` column), not the internal integer PK. Using UUIDs in public URLs prevents sequential enumeration of resources. Internal integer PKs are not exposed in any API response.

### Brands

- `GET /brands`
  - Filters: `relationship`, `tier`, `is_active`, `q`.
- `GET /brands/{brand_id}` — `brand_id` is the public UUID
- `POST /brands` *(Post-MVP — server is read-only in MVP scope)*
- `PATCH /brands/{brand_id}` *(Post-MVP — server is read-only in MVP scope)*

### Products

- `GET /products`
  - Filters: `brand_id`, `tier`, `relationship`, `updated_after`.
- `GET /products/{product_id}` — `product_id` is the public UUID
- `GET /products/{product_id}/variants`

### Pricing

- `GET /pricing/snapshots`
  - Filters: `brand_id`, `variant_id`, `from`, `to`.
- `GET /pricing/summary`
  - Aggregates by brand, dosage, and timeframe.

### Collection Runs

- `GET /collection-runs`
  - Filters: `run_type`, `status`, `from`, `to`.
- `POST /collection-runs/products` *(Post-MVP — server is read-only in MVP scope; use CLI for collection)*
  - Triggers product collection.
- `POST /collection-runs/pricing` *(Post-MVP — server is read-only in MVP scope; use CLI for collection)*
  - Triggers pricing snapshot collection.
- `POST /collection-runs/regs` *(Post-MVP — server is read-only in MVP scope; use CLI for collection)*
  - Triggers legislative collection.

### Regulatory

- `GET /bills`
  - Filters: `jurisdiction`, `status`, `q`.
- `GET /bills/{bill_id}`
- `GET /bills/{bill_id}/events`

### Sentiment

- `GET /sentiment/summary`
  - Returns most recent snapshot per active brand, ordered by brand name.
  - Fields: `brand_name`, `brand_slug`, `score` (string-encoded decimal), `signal_count`, `captured_at`.
- `GET /sentiment/snapshots`
  - Query params: `limit` (default `50`, max `200`).
  - Returns recent snapshots across all brands, ordered by `captured_at DESC`.
  - Fields: same as summary.

## Scope Status

### Implemented

- API versioning convention (`/api/v1`)
- Auth convention (`Authorization: Bearer <api_key>`)
- Response envelope format
- `GET /health`
- `GET /products`
- `GET /pricing/snapshots`, `GET /pricing/summary`
- `GET /bills`, `GET /bills/{bill_id}/events`
- `GET /sentiment/summary`, `GET /sentiment/snapshots`

### Planned Post-MVP / Future Work

- Brand and product detail endpoints
- Idempotency key enforcement across write operations
- OpenAPI generation and publication endpoint

## HTTP Status Codes

- `200 OK`: read success
- `201 Created`: resource created
- `202 Accepted`: async collection job queued
- `400 Bad Request`: invalid params
- `401 Unauthorized`: missing/invalid API key
- `404 Not Found`: missing resource
- `409 Conflict`: duplicate or invalid state transition
- `429 Too Many Requests`: rate limit hit
- `500 Internal Server Error`: unhandled failure

## Idempotency

- Create/update write operations should support `Idempotency-Key` header where replay safety matters.

## OpenAPI

- Server crate should expose generated OpenAPI spec at `/api/v1/openapi.json`.
- Keep request/response schemas in sync with `scbdb-core` types.
