# SCBDB Product Requirements Document

## Document Control

- Version: 1.2
- Owner: Reyes Beverage Group
- Last Updated (EST): 18:55:35 | 02/18/2026 EST
- Status: Active

## 1. Executive Summary

SCBDB is a competitive intelligence and regulatory tracking platform for hemp-derived THC beverages. The product is built as a Rust workspace with a CLI-first workflow and an API/web dashboard path for later phases.

The system helps the team answer four recurring questions:

1. What changed across competitor product catalogs and pricing?
2. Where are portfolio brands over- or under-indexed versus the market?
3. What is the current legislative risk in South Carolina and federal channels?
4. What actions should distribution and commercial teams take this week?

## 2. Product Goals

### 2.1 Primary Goals

- Maintain structured, current competitor and portfolio brand product intelligence.
- Track market gaps across dosage, format, flavor, and pricing bands.
- Track legislation and policy movement with operational impact context.
- Produce repeatable outputs for leadership and field teams.

### 2.2 Success Metrics

- Tier 1 brands fully ingested with product and variant coverage.
- Weekly data refresh success rate >= 95%.
- Legislative event updates posted within 48 hours of movement.
- Core report generation time < 60 seconds for standard datasets.

## 3. Scope

### 3.1 In Scope

- Brand registry and classification (`portfolio` vs `competitor`) in `config/brands.yaml`.
- Shopify product ingestion and normalization.
- Pricing snapshots and historical comparison.
- Legislative ingestion and status timelines.
- CLI commands for collection, status checks, and report generation.
- API design for dashboard and integrations.

### 3.2 Out of Scope for MVP

- Vector search and semantic question answering.
- Embedding generation and vector indexing.
- Generic non-Shopify crawler fallback.
- Alerting and notification automation.
- Multi-state expansion beyond configured targets.

### 3.3 Sentiment Infrastructure (Phase 4)

The following components are integrated in the Phase 4 sentiment pipeline:

- Qdrant (semantic vector index for signal dedup and future similarity search)
- TEI (embedding generation via Qwen3-Embedding-0.6B)

### 3.4 Post-MVP Planned Components

The following components remain part of the long-term product strategy and are intentionally deferred until after MVP is stable:

- Spider (fallback crawling for non-Shopify sources)

## 4. User Personas

### 4.1 Platform Operator

- Runs ingestion jobs
- Maintains config and brand coverage
- Validates data quality

### 4.2 Commercial Stakeholder

- Consumes periodic insights
- Reviews comparison reports
- Uses output for account and portfolio decisions

### 4.3 Regulatory Stakeholder

- Monitors bill status and timelines
- Assesses policy risk and operational impact

## 5. Functional Requirements

### 5.1 Brand Management

- Store all tracked brands in `config/brands.yaml`.
- Support relationship classification (`portfolio`, `competitor`).
- Support prioritization tier (`1`, `2`, `3`).
- Support domain, collection URL, and operational notes.

### 5.2 Product Intelligence

- Pull products and variants from Shopify stores.
- Normalize SKUs, prices, dosage, format, and flavor metadata.
- Persist source timestamps and collection metadata.
- Store repeated snapshots for trend analysis.

### 5.3 Regulatory Tracking

- Ingest bills, status updates, and timeline events.
- Persist source references and effective dates.
- Expose current status and recent movement summaries.

### 5.4 Reporting

- Generate markdown and CSV outputs from structured data.
- Compare portfolio brands to competitors by selected dimensions.
- Produce periodic snapshot reports for internal sharing.

### 5.5 API Surface

- Expose read endpoints for brands, products, pricing, and bills.
- Expose write endpoints for controlled operations (collection triggers).
- Return consistent error responses with request IDs.

## 6. Non-Functional Requirements

- Rust async runtime with reliable I/O and concurrency.
- PostgreSQL-backed persistence using sqlx.
- Clear observability via tracing-based logs.
- Deterministic migrations and reproducible local setup.
- API key-based auth for non-public endpoints.

## 7. Architecture Summary

- Workspace crates: CLI, server, core, db, scraper, legiscan, sentiment.
- Frontend: React + TypeScript + Vite + TanStack Query.
- Data store: PostgreSQL.
- Deployment: self-hosted Docker environments.

Authoritative architecture details live in:

- `ARCHITECTURE.md`
- `TECHNOLOGY.md`
- `DATABASE_SCHEMA.md`
- `API_DESIGN.md`

## 8. Delivery Plan

Execution phases are split into dedicated docs and tracked independently:

- `mvp_phases/phase-1-foundation.md`
- `mvp_phases/phase-2-collection-cli.md`
- `mvp_phases/phase-3-regulatory-tracking.md`
- `mvp_phases/phase-4-sentiment-pipeline.md`
- `mvp_phases/phase-5-api-dashboard.md`

## 9. Risks

- Source schema drift in competitor storefronts.
- Legislative feed inconsistency and delayed updates.
- Data normalization quality across brands with inconsistent naming.
- Over-expanding scope before MVP reliability is proven.

## 10. Decisions

- `config/brands.yaml` is the single brand registry (no separate competitor file).
- Brand relationship is data-driven and can support white-label usage.
- Qdrant/TEI are integrated in Phase 4 for sentiment signal dedup and embedding storage.
- Spider remains in roadmap but not MVP implementation scope.
