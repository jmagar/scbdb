# SCBDB

## Document Metadata

- Version: 1.1
- Status: Active
- Last Updated (EST): 18:55:35 | 02/18/2026 EST

SCBDB is a self-hosted competitive intelligence and regulatory tracking platform for hemp-derived THC beverages.

## What It Does

- Tracks brands from a single `config/brands.yaml` registry.
- Collects product and pricing data from Shopify storefronts.
- Tracks legislative activity and bill timelines.
- Generates decision-ready outputs for portfolio and competitor analysis.

## Current Scope

- Rust Cargo workspace backend (`cli`, `server`, shared crates).
- PostgreSQL persistence with sqlx migrations.
- API design and phased delivery docs.
- CLI-first MVP execution.

Post-MVP roadmap includes Spider, Qdrant, and TEI integration.

## Quick Start

1. Copy `.env.example` to `.env` and set values.
2. Start local services:

```bash
docker compose up -d
```

3. Run checks/tests:

```bash
just check
just test
```

4. Run migrations:

```bash
just migrate
```

## Core Docs

- Documentation index: `docs/INDEX.md`
- Product requirements: `docs/PRD.md`
- Architecture: `docs/ARCHITECTURE.md`
- Technology stack: `docs/TECHNOLOGY.md`
- Database schema: `docs/DATABASE_SCHEMA.md`
- API design: `docs/API_DESIGN.md`
- Config loading strategy: `docs/CONFIG_LOADING.md`
- MVP index and phases: `docs/MVP.md`
- Deployment runbook: `docs/DEPLOYMENT.md`
- Development standards: `docs/DEVELOPMENT.md`
- Logging and errors: `docs/LOGGING.md`
- Extraction prompt schema: `docs/EXTRACTION_PROMPT_SCHEMA.md`
