# Miracle International — Platform API

Backend for the **Integrated Global Trade & Business Solutions Platform**: sourcing,
quotations, wholesale orders, imports, payments, documents and business services.

Rust · Axum · Tokio · PostgreSQL (SQLx) · Redis · utoipa (OpenAPI)

> Architecture, conventions and design decisions: **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)**.
> Read it before adding a module.

---

## Quick start

Prerequisites: stable Rust (see `rust-version` in `Cargo.toml`), Docker.

```bash
cp .env.example .env              # local defaults; never commit .env
docker compose up -d              # PostgreSQL 18, Redis 8, MinIO
cargo run                         # applies migrations (DATABASE_RUN_MIGRATIONS=true) and serves :8080
```

| URL | Purpose |
|---|---|
| `GET /health/live` | Liveness (process up) |
| `GET /health/ready` | Readiness (PostgreSQL + Redis reachable) |
| `/api/v1/...` | Versioned REST API |
| `/docs` | Swagger UI (only when `APP_API_DOCS_ENABLED=true`; default on in development only) |
| `/openapi.json` | OpenAPI document (same condition) |

Run the whole stack in containers instead: `docker compose --profile app up -d --build`.

## Everyday commands

```bash
cargo fmt --all                                   # format
cargo clippy --all-targets -- -D warnings         # lint (CI fails on any warning)
cargo test                                        # unit + infrastructure-free API tests

# Full suite including PostgreSQL-backed tests (uses a TEST database server):
DATABASE_URL=postgres://miracle:miracle@localhost:5432/miracle_test \
  cargo test -- --include-ignored

cargo run -- migrate                              # apply migrations and exit
cargo build --release
```

`cargo nextest run -- --include-ignored` works as a faster drop-in test runner.

## Architectural rules (summary)

1. **Modular monolith.** Business code lives in `src/modules/<module>/`. Each module owns its
   domain types, use cases, SQL, HTTP handlers, DTOs and errors.
2. **Modules talk through facades.** Use another module only through what its `mod.rs`
   re-exports. Never reach into its internals, and never query another module's tables.
3. **No cycles.** Upstream modules (`users`, `permissions`, `audit`, `customers`) never import
   downstream ones (`quotations`, `orders`, `payments`, ...).
4. **`shared/` has no business knowledge** and never imports from `modules/`.
5. **Business rules live in domain types**, not handlers. Handlers extract, validate, call one
   use case and map the result.
6. **Every protected operation checks a permission** (`CurrentUser::require` / `scope`) in the
   backend. Never rely on the frontend.
7. **Money is `rust_decimal` + currency** (`shared::money::Money`), stored as `NUMERIC(19,4)`.
   Never `f32`/`f64`.
8. **Multi-step workflows run in one transaction**, including their audit record and outbox job.
9. **Migrations are the only schema change mechanism** and are immutable once deployed.
10. **Never leak internals**: database rows are mapped to domain types, domain types to response
    DTOs. Server errors return `INTERNAL_ERROR` with a `request_id`; details go to logs only.
11. **No `unwrap()` / `println!` in production code** (enforced by clippy lints).

## Repository layout

```
src/
  main.rs           process entry: config, telemetry, server, worker, graceful shutdown
  lib.rs            crate root (lets integration tests use the app)
  app.rs            AppState + router assembly (composition root)
  config/           typed environment configuration, Secret wrapper
  database/         pool, migration runner
  middleware/       request id, tracing, CORS, security headers, timeouts, body limits
  telemetry.rs      tracing subscriber (pretty / JSON)
  openapi.rs        OpenAPI document
  jobs/             PostgreSQL-backed job queue (transactional outbox) + worker
  shared/           error envelope, responses, pagination, money, IDs, validation,
                    auth primitives (JWT, Argon2, permissions, CurrentUser), storage trait
  modules/          business modules (see docs/ARCHITECTURE.md)
migrations/         SQLx migrations (0001_*.sql ...)
tests/api/          HTTP-level tests against the real router
docker/             local container support files
```

## Configuration

All configuration comes from environment variables; see [`.env.example`](.env.example).
Startup fails fast on missing or invalid critical values (e.g. `JWT_SECRET` shorter than
32 bytes, wildcard CORS outside development). Secrets are wrapped in `config::Secret`,
which never prints its value.
