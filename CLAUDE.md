# CLAUDE.md

## Document Metadata

- Version: 1.1
- Status: Active
- Last Updated (EST): 18:55:35 | 02/18/2026 EST

## Project Identity

SCBDB is a self-hosted competitive intelligence and regulatory tracking platform for hemp-derived THC beverages.

## Stack Snapshot

- Backend: Rust workspace (`scbdb-cli`, `scbdb-server`, shared crates)
- API: Axum + tower middleware
- DB: PostgreSQL + sqlx migrations
- Frontend: React 19 + TypeScript + Vite
- Styling/UI: Tailwind v4 + shadcn/ui

## Product Scope

- Single brand registry: `config/brands.yaml`
- Brand relationship model: `portfolio` or `competitor`
- Tier model: `1`, `2`, `3`
- Core capabilities: product intelligence, pricing history, regulatory tracking, reporting
- Post-MVP roadmap: Spider, Qdrant, TEI

## Canonical Docs

- `docs/INDEX.md`
- `docs/PRD.md`
- `docs/ARCHITECTURE.md`
- `docs/TECHNOLOGY.md`
- `docs/DATABASE_SCHEMA.md`
- `docs/API_DESIGN.md`
- `docs/CONFIG_LOADING.md`
- `docs/MVP.md`
- `docs/DEPLOYMENT.md`
- `docs/DEVELOPMENT.md`
- `docs/LOGGING.md`

## Development Rules

- CLI-first delivery for MVP phases.
- Use typed configuration and fail fast on startup errors.
- Validate external data at boundaries.
- Keep migrations append-only and schema docs current.
- Keep docs aligned to implemented architecture; no historical-stack references in active docs.

## Documentation Workflow

When project behavior or architecture changes:

1. Update canonical technical docs first.
2. Update `docs/PRD.md` if product requirements changed.
3. Update phase docs if delivery sequencing changed.
4. Keep this file synchronized with current state.
