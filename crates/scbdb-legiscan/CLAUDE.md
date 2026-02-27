# CLAUDE.md — scbdb-legiscan

LegiScan API client and bill ingestion library for SCBDB regulatory tracking.

## Purpose

Provides a typed, budget-aware HTTP client for the [LegiScan REST API](https://legiscan.com/legiscan)
and a normalization layer that converts raw API responses into domain structs ready for
database persistence via `scbdb-db`.

## Crate Type

Library (`lib`). Consumed exclusively by `scbdb-cli` (`crates/scbdb-cli/src/regs/`).
Does **not** call `dotenvy::dotenv()` — that is the binary's responsibility.

## Public API

```rust
// Client
LegiscanClient::new(api_key, timeout_secs, max_requests)
LegiscanClient::with_base_url(api_key, timeout_secs, max_requests, base_url) // for tests
client.get_bill(bill_id)              // getBill endpoint
client.search_bills(query, state, max_pages) // search endpoint (paginated)
client.get_session_list(state)        // getSessionList endpoint
client.get_master_list(state)         // getMasterList (current session)
client.get_master_list_by_session(session_id) // getMasterList (historical)
client.requests_used()               // monotonic counter for budget tracking

// Normalization — BillDetail → DB-ready structs
normalize_bill(detail)        -> NormalizedBill
normalize_bill_events(detail) -> Vec<NormalizedBillEvent>
normalize_bill_texts(detail)  -> Vec<NormalizedBillText>

// Error
LegiscanError::{Http, ApiError, Deserialize, QuotaExceeded, BudgetExceeded}
```

## Environment Variables

| Variable | Required | Purpose |
|----------|----------|---------|
| `LEGISCAN_API_KEY` | Yes | LegiScan API authentication |

The key is accessed via `scbdb_core::AppConfig.legiscan_api_key` — the crate itself does not
read env vars directly.

## LegiScan Endpoints Used

All requests are GET to `https://api.legiscan.com/?key=<KEY>&op=<OP>&...`.

| Method | `op` param | Extra params | Returns |
|--------|-----------|-------------|---------|
| `get_bill` | `getBill` | `id=<bill_id>` | `BillDetail` |
| `search_bills` | `search` | `query=`, `state=`, `page=` | `Vec<BillSearchItem>` (paginated, 50/page) |
| `get_session_list` | `getSessionList` | `state=` | `Vec<SessionInfo>` |
| `get_master_list` | `getMasterList` | `state=` | `(SessionDetail, Vec<MasterListEntry>)` |
| `get_master_list_by_session` | `getMasterList` | `id=<session_id>` | same |

Every response is wrapped in `{"status": "OK" | "ERROR", ...}`. `check_api_error()` inspects
`status` and the `alert.message` field before any typed deserialization occurs.

## Key Type: MasterListEntry

`MasterListEntry` is the per-bill summary returned by `getMasterList`. Key fields:

```rust
pub struct MasterListEntry {
    pub bill_id: i64,            // LegiScan's stable numeric ID for the bill
    pub change_hash: String,     // MD5 of bill content — used for skip-unchanged check
    pub bill_number: String,     // e.g. "HB1234"
    pub title: String,           // short title
    pub status: i32,             // integer status code (see Bill Status Codes below)
    pub last_action: String,     // human-readable last action description
    pub last_action_date: Option<String>, // "YYYY-MM-DD" or null
}
```

The `change_hash` field is the deduplication keystone: Phase 2 of ingestion compares incoming hashes against `bills.legiscan_change_hash` in the DB to skip unchanged bills entirely.

## The Numbered-Key JSON Pattern

**This is the most important gotcha in this crate.**

LegiScan returns list payloads as numbered string keys, not JSON arrays:

```json
{
  "status": "OK",
  "searchresult": {
    "summary": { ... },
    "0": { "bill_id": 1, "title": "..." },
    "1": { "bill_id": 2, "title": "..." }
  }
}
```

The same shape appears in **both** `search` and `getMasterList` responses.

**Deserialization strategy:**

```rust
#[derive(Deserialize)]
pub struct SearchResult {
    pub summary: SearchSummary,
    #[serde(flatten)]
    pub results: HashMap<String, serde_json::Value>,  // captures "0", "1", ...
}
```

Then at the call site, filter numeric keys and deserialize individually:

```rust
let items: Vec<BillSearchItem> = envelope
    .data
    .searchresult
    .results
    .into_iter()
    .filter(|(k, _)| k.parse::<u32>().is_ok())  // drop "summary", "session", etc.
    .filter_map(|(k, v)| {
        serde_json::from_value::<BillSearchItem>(v)
            .map_err(|e| tracing::warn!(key=%k, error=%e, "skipping malformed entry"))
            .ok()
    })
    .collect();
```

`MasterListData` uses the same pattern but also has a `"session"` key to exclude (not a
numeric key, so `k.parse::<u32>().is_ok()` handles it automatically — but the explicit
`filter(|(k, _)| k != "session")` guard in `session.rs` is belt-and-suspenders).

## Type System Quirks

**`de_opt_int` custom deserializer** (`types.rs`):

LegiScan sometimes returns integer fields as JSON strings (`"1"` instead of `1`).
`SearchSummary.page_current`, `SearchSummary.page_total`, and `BillSearchItem.status`
all use the `#[serde(deserialize_with = "de_opt_int::deserialize")]` attribute to
handle both representations transparently.

**`SearchSummary` string fields:**

`page` (`"1 of 451"`), `range` (`"1 - 50"`), and `relevancy` (`"100% - 99%"`) are
human-readable strings, **not** integers. Treating them as `i32` causes deserialization
failure. They are typed as `String`.

## Bill Status Codes

| Code | String |
|------|--------|
| 1 | `introduced` |
| 2 | `engrossed` |
| 3 | `enrolled` |
| 4 | `passed` |
| 5 | `vetoed` |
| 6 | `failed` |
| other | `unknown(<N>)` |

Conversion is via `normalize::map_status(i32) -> String`.

## Rate Limiting and Budget Enforcement

**Two separate protections:**

1. **Per-session budget** (`max_requests: u32`, set at client construction).
   Tracked with an `AtomicU32`. Once `requests_used >= max_requests`, all further
   calls return `LegiscanError::BudgetExceeded` immediately — no HTTP traffic.
   Budget counts the logical call, not individual retry attempts.

2. **API quota** (`LegiscanError::QuotaExceeded`). When LegiScan returns
   `"status": "ERROR"` with a message containing `"limit exceeded"` or
   `"access denied"`, the error is `QuotaExceeded` — a hard stop that callers
   must surface immediately without retrying.

**Retry policy** (`retry.rs`):

Exponential backoff with ±25% jitter, capped at 60 s, up to 3 retries.
Retriable: `reqwest` timeouts, connect errors, HTTP 5xx.
Non-retriable (returned immediately): `QuotaExceeded`, `BudgetExceeded`,
`ApiError`, `Deserialize`.

Default CLI configuration: `--max-requests 5000` (supports ~150 daily runs/month
against the 30 000/month plan, leaving headroom for ad-hoc use).

## Bill Ingestion Pipeline

The pipeline lives in `scbdb-cli/src/regs/` and calls into this crate in three phases:

```
Phase 1 — Discovery
  getMasterList(state)                 # 1 request/state (current session)
  OR getSessionList + getMasterList×N  # all-sessions backfill mode
  → local keyword filter on bill title (no API cost)
  → CandidateMap: bill_id → (change_hash, MasterListEntry)

Phase 2 — Hash check (no API requests)
  scbdb_db::get_bills_stored_hashes(pool, &all_ids)
  → skip bills where incoming change_hash == stored legiscan_change_hash

Phase 3 — Fetch + upsert (1 request/changed bill)
  getBill(bill_id)
  → normalize_bill() + normalize_bill_events() + normalize_bill_texts()
  → scbdb_db::upsert_bill()
  → scbdb_db::upsert_bill_event() per event
  → scbdb_db::upsert_bill_text() per text (idempotent: ON CONFLICT DO NOTHING on legiscan_text_id)
```

Steady-state cost is `N_states + N_changed_bills` requests (not `N_bills`).

## Dry-Run Mode

Dry-run is handled at the CLI layer (`regs/ingest.rs`), not inside this crate.
When `--dry-run` is set, `run_regs_ingest` prints what would be ingested and
returns before constructing the `LegiscanClient`. This crate has no dry-run logic.

## Database Tables

| Table | Key column | Dedup strategy |
|-------|-----------|----------------|
| `bills` | `(jurisdiction, bill_number)` UNIQUE | `ON CONFLICT DO UPDATE` (upsert) |
| `bills.legiscan_bill_id` | `BIGINT UNIQUE` | Cross-ref key for hash check |
| `bills.legiscan_change_hash` | `TEXT` | Skip-unchanged optimization |
| `bill_events` | `(bill_id, event_date, description)` | Unique constraint (migration 000400) |
| `bill_texts` | `legiscan_text_id` UNIQUE | `ON CONFLICT DO NOTHING` (immutable) |

`bill_texts` entries are immutable once created — `doc_id` is stable and a second
ingest of the same text version produces no change.

## Testing

Tests use `wiremock` for HTTP mocking. The `with_base_url` constructor is the
test entry point — it accepts any URL, including the `wiremock::MockServer` URI.

```rust
// Pattern for all integration tests
let server = wiremock::MockServer::start().await;
wiremock::Mock::given(wiremock::matchers::query_param("op", "getBill"))
    .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(fixture))
    .mount(&server)
    .await;
let client = LegiscanClient::with_base_url("test-key", 30, 1_000, &server.uri()).unwrap();
```

Unit tests for `normalize`, `retry`, and URL construction are inline in their
respective modules (no wiremock needed — no network calls).

Run this crate's tests only:

```bash
cargo test -p scbdb-legiscan
```

## Module Map

| File | Contents |
|------|---------|
| `lib.rs` | Public re-exports |
| `client.rs` | `LegiscanClient` struct, `get_bill`, `search_bills`, `request_json`, `check_api_error` |
| `session.rs` | `get_session_list`, `get_master_list`, `get_master_list_by_session`, `parse_master_list_response` |
| `types.rs` | All `#[derive(Deserialize)]` types mirroring the LegiScan wire format |
| `normalize.rs` | `NormalizedBill`, `NormalizedBillEvent`, `NormalizedBillText`; `normalize_bill/events/texts`, `map_status`, `parse_date` |
| `error.rs` | `LegiscanError` enum |
| `retry.rs` | `retry_with_backoff`, `is_retriable` |
| `client_test.rs` | `build_url` unit tests (pulled into `client.rs` via `#[path]`) |

## Workspace Dependencies

This crate depends only on external crates (no `scbdb-*` workspace crates).
It is consumed by `scbdb-cli` which brings in `scbdb-db` and `scbdb-core`.

| Dependency | Use |
|-----------|-----|
| `reqwest` | HTTP client |
| `serde` + `serde_json` | Deserialization of API responses |
| `thiserror` | Error type derivation |
| `tokio` | Async runtime |
| `tracing` | Structured logging |
| `chrono` | `NaiveDate` parsing in normalize layer |
| `rand` | Jitter in retry backoff |
| `wiremock` (dev) | HTTP mocking in tests |

## Conventions

- All public `async fn` on `LegiscanClient` are documented with `# Errors` listing
  every `LegiscanError` variant that can be returned.
- `#[must_use]` on pure functions in `normalize.rs` and `client.rs`.
- Internal helpers (`retry`, `session`) are `pub(crate)` — not part of the public API.
- Budget check (`BudgetExceeded`) happens before any HTTP attempt — the count
  is incremented optimistically and rolled back if the ceiling is already reached.
- State abbreviations are uppercased at the call site before sending to the API
  (`"sc"` → `"SC"`). The API rejects lowercase state codes.
