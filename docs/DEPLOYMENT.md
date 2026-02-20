# Deployment

## Document Metadata

- Version: 1.0
- Status: Draft
- Last Updated (EST): 18:55:35 | 02/18/2026 EST

## Purpose

Define the deployment and operations baseline for self-hosted SCBDB environments.

## Environment Model

- Runtime target: self-hosted Docker environment.
- Database: PostgreSQL container/service.
- App services: `scbdb-server` and supporting workers/CLI runners.
- Frontend: Vite-built static assets served by API/static host strategy (to be finalized).

## Deployment Topology (Baseline)

1. Build application artifacts (Rust binaries and frontend assets).
2. Build container images.
3. Apply database migrations.
4. Deploy updated containers.
5. Run smoke checks (`/api/v1/health`, DB connectivity, auth checks).

## Configuration and Secrets

- Runtime config via environment variables.
- Use `.env`-style files for development; production secret store strategy is pending.
- Required variables are defined in `.env.example` and `docs/CONFIG_LOADING.md`.

## Operational Checks

- Health endpoint responds successfully.
- Migration version is current.
- Collection jobs can be queued and completed.
- Logs contain request IDs and no secret leakage.

## Rollback Strategy

- Keep previously working container images available.
- Roll back app containers first if release health checks fail.
- Use reversible migrations for risky schema changes where possible.

## Open Items

- Finalize production service composition file(s).
- Decide static asset serving strategy for frontend.
- Define backup/restore procedure for PostgreSQL.
- Define alerting and uptime checks.
