# Configuration Loading Strategy

## Document Metadata

- Version: 1.1
- Status: Active
- Last Updated (EST): 18:55:35 | 02/18/2026 EST

## Goals

- Single, predictable configuration flow across CLI and server.
- Strong validation at startup.
- Explicit precedence with all defaults documented.

## Sources

1. Environment variables (`.env` in development).
2. `config/brands.yaml` brand registry file.
3. Optional runtime overrides through CLI flags.

## Precedence

1. CLI flags (highest priority)
2. Environment variables
3. `.env` file values
4. Hardcoded defaults (lowest priority)

## Required Environment Variables

- `DATABASE_URL` — full PostgreSQL connection string read by the application
- `SCBDB_ENV` (`development`, `test`, `production`)

## Docker Compose Variables

- `POSTGRES_PASSWORD` — required by the `docker-compose.yml` `postgres` service to initialize the container; not read by the application (which uses `DATABASE_URL` instead)

## Optional Environment Variables

- `SCBDB_BIND_ADDR` (default `0.0.0.0:3000`)
- `SCBDB_LOG_LEVEL` (default `info`)
- `SCBDB_BRANDS_PATH` (default `./config/brands.yaml`)
- `LEGISCAN_API_KEY`

## Rust Ownership

- `scbdb-core`: defines typed config models and validation.
- `scbdb-cli`: loads env + file config for command execution.
- `scbdb-server`: loads env + file config at startup and fails fast if invalid.

## Loading Sequence

1. Call `dotenvy::dotenv()` in binaries.
2. Parse env into typed struct in `scbdb-core`.
3. Resolve brands file path.
4. Parse `config/brands.yaml`.
5. Validate schema and business rules.
6. Start runtime only after validation passes.

## Validation Rules

- Brand names and slugs must be unique.
- `relationship` must be `portfolio` or `competitor`.
- `tier` must be `1`, `2`, or `3`.
- `shop_url` required for scrape-targeted brands.
- Domain format must be valid when provided.

## Failure Behavior

- Config parse/validation errors are fatal at startup.
- Error output includes actionable context (field + reason).
- No partial startup with invalid config.

## Future Extensions

- Add a tracked-bills seed config file if manual bill bootstrapping becomes necessary.
- Add environment-specific config files if needed (`config/*.yaml`).
- Add secret manager adapter if homelab secret store is introduced.
