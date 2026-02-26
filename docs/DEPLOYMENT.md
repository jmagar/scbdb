# Deployment

## Document Metadata

- Version: 2.0
- Status: Active
- Last Updated (EST): 18:00:00 | 02/24/2026 EST

## Purpose

Deployment and operations guide for self-hosted SCBDB environments.

## Prerequisites

| Dependency | Version | Notes |
|------------|---------|-------|
| Rust | stable (latest) | `rustup` recommended |
| Node.js | >=20.19.0 | Required for web frontend build |
| pnpm | 10+ | Frontend package manager |
| Docker | 24+ | For PostgreSQL and optional containerized deployment |
| Docker Compose | v2+ | `docker compose` (not the deprecated `docker-compose`) |
| PostgreSQL | 16+ | Provided via Docker Compose or external instance |

## Building

### Backend (Rust)

```bash
cargo build --release -p scbdb-server -p scbdb-cli
```

Release binaries are written to `target/release/scbdb-server` and `target/release/scbdb-cli`.

### Frontend (Web)

```bash
pnpm --dir web install
pnpm --dir web build
```

Output is written to `web/dist/`. The server serves these as static assets.

## Docker Deployment

The repository includes a `docker-compose.yml` at the project root. It defines a `postgres` service pre-configured for SCBDB:

```bash
cp .env.example .env    # edit with real values
docker compose up -d    # starts PostgreSQL
```

The compose file reads `POSTGRES_PASSWORD`, `POSTGRES_USER`, `POSTGRES_DB`, and `POSTGRES_PORT` from `.env`. The default host port is `15432` to avoid conflicts with any system PostgreSQL.

The application binaries (`scbdb-server`, `scbdb-cli`) run directly on the host. Containerizing them is straightforward but not provided out of the box — the current model is bare-metal binaries talking to a Dockerized PostgreSQL.

## Environment Variables

All configuration is via environment variables. See [`.env.example`](../.env.example) for the full list with defaults and comments.

Critical variables:

| Variable | Required | Purpose |
|----------|----------|---------|
| `DATABASE_URL` | Yes | PostgreSQL connection string |
| `POSTGRES_PASSWORD` | Yes | Database password (used by Docker Compose and sqlx) |
| `SCBDB_API_KEYS` | Production | Comma-separated bearer tokens for API auth |
| `SCBDB_ENV` | No | Set to `production` to enforce mandatory auth; defaults to `development` |
| `SCBDB_BIND_ADDR` | No | Server bind address; defaults to `0.0.0.0:3000` |

## Database Setup

### Quick path (recommended)

```bash
just bootstrap    # db-up → wait → migrate → ping → seed
```

This starts the PostgreSQL container, waits for readiness, applies all migrations, verifies connectivity, and seeds brands from `config/brands.yaml`.

### Manual path

1. Start PostgreSQL (via Docker Compose or external instance).
2. Create the database and user if not using the compose defaults.
3. Apply migrations:

```bash
cargo run -p scbdb-cli -- db migrate
```

4. Verify connectivity:

```bash
cargo run -p scbdb-cli -- db ping
```

5. Seed brand data:

```bash
cargo run -p scbdb-cli -- db seed
```

## Running the Server

```bash
cargo run --release -p scbdb-server
```

Or with explicit environment:

```bash
DATABASE_URL=postgres://scbdb:changeme@localhost:15432/scbdb \
SCBDB_BIND_ADDR=0.0.0.0:3000 \
SCBDB_ENV=production \
SCBDB_API_KEYS=your-strong-token-here \
./target/release/scbdb-server
```

The server applies pending migrations automatically on startup, then begins serving.

## Secrets Management

- **Never commit `.env`** — it is gitignored. Only `.env.example` is tracked.
- Use strong, random API keys in production (e.g. `openssl rand -base64 32`).
- Rotate `SCBDB_API_KEYS` periodically. The server reads them on startup, so rotation requires a restart.
- Keep `POSTGRES_PASSWORD` strong and unique per environment.

## Reverse Proxy and TLS

The server binds plain HTTP. It does **not** terminate TLS itself.

For production, place a reverse proxy in front of `scbdb-server`:

- **Caddy** — automatic HTTPS with Let's Encrypt, minimal config.
- **nginx** — standard reverse proxy with `proxy_pass`.
- **SWAG** (Secure Web Application Gateway) — nginx + Let's Encrypt in a Docker container.

The proxy should forward `X-Forwarded-For` and `X-Request-Id` headers. Rate limiting keys off these when auth is disabled.

## Health Check

```bash
curl http://localhost:3000/api/v1/health
```

Returns `200 OK` with `{"data":{"status":"ok","database":"ok"}}` when the database is connected. Returns `503 Service Unavailable` with `"database":"unavailable"` when the pool cannot reach PostgreSQL.

Use this endpoint for uptime monitoring, Docker health checks, and reverse proxy health probes.

## Backup

PostgreSQL data is the only stateful component. Back it up with `pg_dump`:

```bash
# Full logical backup
pg_dump -h localhost -p 15432 -U scbdb -d scbdb -Fc -f scbdb_backup.dump

# Restore
pg_restore -h localhost -p 15432 -U scbdb -d scbdb --clean scbdb_backup.dump
```

Automate with cron for production environments. Keep at least 7 days of daily backups.

## Rollback Strategy

- Keep the previous release binary available. If a deploy fails health checks, swap back to the previous binary and restart.
- Database migrations are append-only. For risky schema changes, write a reversible migration and test the rollback path before deploying.
- Docker images (PostgreSQL) are pinned by the compose file. Rolling back the app does not require rolling back the database container.
