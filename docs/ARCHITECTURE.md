# Architecture

This document explains how the Miracle International backend is organised and why.
It is the reference for anyone adding a module, an endpoint, a table or a background job.

Contents

1. [Architecture overview](#1-architecture-overview)
2. [Recommended folder tree](#2-recommended-folder-tree)
3. [Top-level folders](#3-top-level-folders)
4. [Module dependency rules](#4-module-dependency-rules)
5. [Example module structures](#5-example-module-structures)
6. [Shared infrastructure strategy](#6-shared-infrastructure-strategy)
7. [PostgreSQL / SQLx strategy](#7-postgresql--sqlx-strategy)
8. [RBAC architecture](#8-rbac-architecture)
9. [Error handling](#9-error-handling)
10. [API responses](#10-api-responses)
11. [Background jobs and events](#11-background-jobs-and-events)
12. [File storage](#12-file-storage)
13. [Logging, observability and auditing](#13-logging-observability-and-auditing)
14. [Testing](#14-testing)
15. [Configuration](#15-configuration)
16. [Cargo dependencies](#16-cargo-dependencies)
17. [Security checklist](#17-security-checklist)
18. [Extracting a module into a service](#18-extracting-a-module-into-a-service)

---

## 1. Architecture overview

**Style:** a modular monolith with domain-oriented modules, using ideas from Clean
Architecture, Hexagonal Architecture and DDD only where they pay for themselves.

```
                 ┌──────────────────────────── one process ────────────────────────────┐
HTTP ──► Tower   │  middleware  ──►  modules/<x>/api  ──►  modules/<x>/application     │
         layers  │  (request id,     (extract, validate,   (use case: permission check, │
                 │   tracing, CORS,   map DTOs)             transaction, domain calls,  │
                 │   timeouts)                             audit, enqueue job)          │
                 │                                             │            │           │
                 │                              modules/<x>/domain     repository (SQLx)│
                 │                              (pure rules, no I/O)          │           │
                 │  jobs::worker ◄── background_jobs (outbox) ◄───────────────┤           │
                 └──────────────────────────────────────────────────────────────┼──────────┘
                                                                  PostgreSQL · Redis · S3
```

Key decisions:

| Decision | Choice | Reason |
|---|---|---|
| Deployment unit | One binary (API + job worker), scaled horizontally | Simple operations; modules can be split later (section 18) |
| Module boundary | A Rust module with a public facade in `mod.rs` | Compiler-enforced visibility, zero runtime cost |
| Persistence | PostgreSQL, SQLx, hand-written SQL | Relational B2B data, transactions, constraints |
| Money | `rust_decimal` + `Currency`, `NUMERIC(19,4)` | Exact arithmetic |
| IDs | UUID v7, one Rust newtype per entity | Unguessable, time-ordered, type-safe |
| Auth | Short-lived JWT access token + rotating opaque refresh token in `user_sessions` | Stateless verification plus immediate revocation |
| Authorization | Permission-based RBAC checked in use cases | Roles change without code changes |
| Async work | PostgreSQL job table (transactional outbox) | Jobs commit or roll back with the business change |
| Docs | utoipa + Swagger UI, per-environment | Contract generated from code |

---

## 2. Recommended folder tree

Items marked ✅ exist in the starter; ◻️ are planned. Folders are created when their first
real code arrives, never as empty placeholders.

```
miracle_backend/
├── Cargo.toml / Cargo.lock / clippy.toml
├── .env.example                      ✅ every supported variable, documented
├── .github/workflows/ci.yml          ✅ fmt, clippy, tests (with Postgres), release build
├── docs/ARCHITECTURE.md              ✅ this file
├── migrations/                       ✅ 0001–0010 (foundation → payments)
├── tests/api/                        ✅ HTTP-level tests
│   ├── main.rs, common/mod.rs
│   └── health_and_errors.rs, sessions.rs, quotations.rs, orders.rs,
│       requirements.rs, suppliers.rs, jobs.rs
└── src/
    ├── main.rs                       ✅ entry point, `migrate` subcommand, graceful shutdown
    ├── lib.rs                        ✅
    ├── app.rs                        ✅ AppState, router assembly
    ├── openapi.rs                    ✅
    ├── telemetry.rs                  ✅
    ├── config/                       ✅ mod.rs, env.rs, secret.rs
    ├── database/                     ✅ mod.rs
    ├── middleware/                   ✅ mod.rs, request_context.rs, security_headers.rs
    │   └── rate_limit.rs             ◻️ Redis token bucket (auth routes first)
    ├── jobs/                         ✅ mod.rs (Job, enqueue), worker.rs, handlers.rs
    ├── shared/
    │   ├── auth/                     ✅ tokens.rs, password.rs, permission.rs, current_user.rs
    │   ├── error.rs                  ✅ AppError + JSON envelope
    │   ├── response.rs               ✅ DataResponse, ListResponse
    │   ├── pagination.rs             ✅ page/limit (cursor type ◻️)
    │   ├── money.rs                  ✅ Money, Currency
    │   ├── ids.rs                    ✅ typed_id! macro, UserId
    │   ├── validation.rs             ✅ ValidatedJson, ValidatedQuery
    │   ├── request_context.rs        ✅ current_request_id()
    │   ├── storage.rs                ✅ ObjectStorage trait (S3 impl ◻️)
    │   └── email.rs / sms.rs         ◻️ only if providers are shared beyond notifications
    └── modules/
        ├── mod.rs                    ✅ api_v1_router()
        ├── health.rs                 ✅
        ├── auth/                     ✅ api/, application/, domain.rs, sessions.rs
        ├── users/                    ✅ api.rs, domain.rs, repository.rs
        ├── permissions/              ✅ repository.rs (admin API ◻️)
        ├── customers/                ✅ facade (profile API ◻️)
        ├── staff/                    ◻️
        ├── suppliers/                ✅ api.rs, domain.rs, verification.rs
        ├── products/                 ◻️ products + categories (one module)
        ├── countries/                ◻️ reference data API (table exists)
        ├── requirements/             ✅ api.rs, domain.rs, dto.rs, repository.rs
        ├── sourcing/                 ◻️ sourcing activity + supplier quotations
        ├── quotations/               ✅ api/, application/, domain/, repository.rs
        ├── orders/                   ✅ api/, application/, domain/
        ├── imports/ exports/         ◻️
        ├── logistics/                ◻️ shipments, tracking events, carrier integrations
        ├── payments/                 ✅ domain.rs, gateway.rs, webhooks.rs (API ◻️)
        ├── invoices/                 ◻️ invoices + receipts
        ├── documents/                ◻️ metadata, permissions, presigned URLs
        ├── notifications/            ✅ EmailSender trait (in-app/SMS ◻️)
        ├── bookings/                 ◻️
        ├── business/                 ◻️ consultation + business setup projects
        ├── investment/               ◻️ opportunities + inquiries
        ├── franchise/ travel/ visa/  ◻️
        ├── it_services/              ◻️
        ├── support/                  ◻️ tickets
        ├── cms/                      ◻️
        ├── search/                   ◻️ search facade (Postgres FTS → Meilisearch later)
        ├── reports/                  ◻️ read-only queries / materialised views
        ├── audit/                    ✅ record() (read API ◻️)
        └── settings/                 ◻️ application settings
```

Grouping guidance: merge closely related capabilities into one module when they share
invariants (products + categories, invoices + receipts, consultation + business setup).
Split when two areas have different owners, different change rates or no shared rules.

---

## 3. Top-level folders

| Folder | Responsibility | Must not contain |
|---|---|---|
| `src/main.rs` | Load config, init telemetry, connect, spawn worker, serve, shut down on SIGTERM/Ctrl+C | Business logic |
| `src/app.rs` | `AppState` and router assembly — the composition root that wires concrete implementations to traits | Handlers |
| `src/config/` | Parse and validate environment variables into typed structs; `Secret` | Reading env vars anywhere else |
| `src/database/` | Pool creation, migration runner, ping | Queries for business data |
| `src/middleware/` | Tower layers that apply to every request | Per-module authorization |
| `src/jobs/` | Job type, enqueue, polling worker, dispatch to modules | Job business logic (lives in modules) |
| `src/shared/` | Cross-cutting building blocks without business knowledge | Anything that knows what a quotation or supplier is |
| `src/modules/` | Business capabilities, one folder/file per module | Cross-module table access |
| `migrations/` | The only authoritative schema definition | Edits to already-deployed files |
| `tests/api/` | Tests through the real router and database | Mocks of our own SQL |

---

## 4. Module dependency rules

```
                 shared  ◄──────────── everything may use shared
                   ▲
     ┌─────────────┼───────────────┐
  users   permissions   audit   customers   notifications     (upstream / foundation)
     ▲         ▲          ▲         ▲
     └── auth ─┘          │         │
                          │         │
      suppliers  requirements  ─────┤
          ▲                         │
          └──── quotations ──► orders ──► (payments, invoices, logistics)   (downstream)
```

1. **Facade only.** `use crate::modules::orders::{create_from_quotation, OrderId}` is fine.
   `use crate::modules::orders::domain::status::...` is not. Items not re-exported from
   `mod.rs` are private (`mod domain;`), so the compiler enforces most of this.
2. **Own your tables.** A module reads and writes only its own tables. Need data from another
   module? Call its facade (`customers::find_id_by_user`). Foreign keys across modules are
   fine — they protect integrity without coupling code.
3. **No cycles.** Downstream modules depend on upstream ones, never the reverse. When an
   upstream module needs to react to a downstream change, use a job/event
   (`Job::QuotationAccepted`) rather than an import.
4. **Cross-module transactions** pass `&mut PgConnection` through the facade
   (`orders::create_from_quotation(&mut tx, input)`). The *initiating* use case owns
   `begin()`/`commit()`.
5. **`shared` never imports `modules`.** When shared code needs module behaviour it defines a
   trait (e.g. `PrincipalResolver`) that a module implements and `app.rs` wires in.
6. **`api` layers are thin.** They may import their own module's application and domain
   types, `shared`, and nothing from other modules' `api` folders.

Review checklist for a PR touching modules: new `use crate::modules::...` lines only name
re-exported items; no SQL mentions another module's table; no new upstream → downstream import.

---

## 5. Example module structures

Pick the smallest layout that keeps each file understandable. Start flat; grow into
directories when a file stops fitting on a screen or holds more than one use case.

### auth (✅ exists)

```
auth/
├── mod.rs              facade: routes, AuthError, PgPrincipalResolver
├── domain.rs           AuthError (+ HTTP mapping), lockout constants
├── sessions.rs         user_sessions persistence; PgPrincipalResolver (session + account + permissions)
├── application/
│   ├── register.rs     RegisterUser     (TODO steps documented inline)
│   ├── login.rs        LoginUser        (TODO: brute-force limiter, lockout, session + tokens)
│   └── logout.rs       LogoutUser       (implemented)
└── api/
    ├── mod.rs          routes
    ├── handlers.rs
    └── dto.rs          RegisterRequest, LoginRequest, TokenResponse
```
Planned: `refresh.rs` (rotation + reuse detection), `verify_email.rs`, `password_reset.rs`,
`revoke_sessions.rs`.

### requirements (✅ exists — flat)

```
requirements/
├── mod.rs
├── domain.rs           RequirementId, RequirementKind, RequirementStatus
├── dto.rs              ListRequirementsQuery (typed filters), SubmitRequirementRequest, responses
├── repository.rs       list() with QueryBuilder + push_bind; LIKE escaping
└── api.rs              GET / (implemented, scoped), POST / (TODO)
```
Grows into `application/submit_requirement.rs`, `assign_requirement.rs` as workflows appear.

### quotations (✅ exists — full layering, the reference module)

```
quotations/
├── mod.rs
├── domain/
│   ├── quotation.rs    Quotation, QuotationItem, QuotationCharge, totals(), send/accept/reject rules
│   ├── status.rs       QuotationStatus + allowed transitions
│   └── errors.rs       QuotationError → AppError
├── application/
│   └── accept_quotation.rs   transactional cross-module workflow (implemented)
├── repository.rs       find_for_update (row lock), update_status (optimistic version)
└── api/
    ├── mod.rs, handlers.rs, dto.rs
```
Planned use cases: `create_quotation.rs`, `update_draft.rs`, `send_quotation.rs`,
`reject_quotation.rs`, `cancel_quotation.rs`, `expire_quotations.rs` (scheduled job).

### orders (✅ exists)

```
orders/
├── mod.rs              facade: routes, create_from_quotation, OrderId, OrderStatus
├── domain/
│   ├── mod.rs          OrderId
│   ├── status.rs       OrderStatus + transition table (tested)
│   └── errors.rs
├── application/
│   ├── create_from_quotation.rs   runs inside the caller's transaction
│   └── update_status.rs           lock → transition → history → audit → commit
└── api/                PATCH /{id}/status
```
There is deliberately **no generic "set status" endpoint**: each change goes through the
transition table, and cancellation additionally needs `order.cancel` and a reason.

### suppliers (✅ exists — flat)

```
suppliers/
├── mod.rs
├── domain.rs           SupplierId, VerificationStatus (+ transitions), SupplierError
├── verification.rs     RecordVerificationDecision (implemented)
└── api.rs              POST /{id}/verification
```

### payments (✅ partial)

```
payments/
├── mod.rs
├── domain.rs           PaymentType, PaymentStatus (movement), ScheduleStatus (settlement), PaymentError
├── gateway.rs          PaymentGateway trait (checkout, verify_webhook, refund)
└── webhooks.rs         record_gateway_event — idempotent via UNIQUE(provider, provider_event_id)
```
Planned: `application/record_payment.rs` (requires `Idempotency-Key`),
`confirm_payment.rs`, `refund_payment.rs`, `create_payment_schedule.rs`, `providers/<name>.rs`,
`api/` including `POST /payments/webhooks/{provider}`.

Payment status is split in two on purpose: a single payment is `PENDING → PAID | FAILED`,
`PAID → REFUNDED`; `PARTIALLY_PAID` describes a schedule installment or order balance, derived
from confirmed payments.

---

## 6. Shared infrastructure strategy

What qualifies for `shared/`: used by several modules, stable, no business vocabulary.

| Item | What it gives modules |
|---|---|
| `error.rs` | `AppError`, `AppResult`, stable codes, JSON envelope, extractor rejection mapping |
| `response.rs` | `DataResponse<T>`, `ListResponse<T>` |
| `pagination.rs` | `PageParams` → sanitised `PageRequest` (default 20, max 100) → `PageMeta` |
| `money.rs` | `Money`, `Currency`; checked add/sub/mul, rounding to minor units, string JSON |
| `ids.rs` | `typed_id!` macro (UUID v7 newtypes with serde/sqlx/utoipa), `UserId` |
| `validation.rs` | `ValidatedJson<T>`, `ValidatedQuery<T>` |
| `auth/` | `TokenService`, Argon2id hashing (blocking pool), `Permission` catalogue, `CurrentUser`, `AccessScope`, `PrincipalResolver` |
| `request_context.rs` | `current_request_id()` anywhere inside a request |
| `storage.rs` | `ObjectStorage` trait |

Abstractions are introduced only at real seams: external providers (`ObjectStorage`,
`PaymentGateway`, `EmailSender`) and dependency inversion that breaks a cycle
(`PrincipalResolver`). Repositories are concrete functions, not traits — see section 7.

---

## 7. PostgreSQL / SQLx strategy

**Schema.** Normalised tables, foreign keys everywhere, `CHECK` constraints for statuses and
invariants (e.g. `SENT` quotations must have `valid_until`), `UNIQUE` constraints for
idempotency (`orders.quotation_id`, `payments.idempotency_key`,
`payment_gateway_events(provider, provider_event_id)`), `updated_at` triggers, append-only
trigger on `audit_logs`. JSONB is used only for audit metadata, job payloads and raw webhook
payloads.

**Conventions**

| Topic | Convention |
|---|---|
| Primary keys | `UUID`, generated in Rust as v7 |
| Timestamps | `TIMESTAMPTZ`, UTC |
| Money | `NUMERIC(19,4)` + `CHAR(3)` → `currencies(code)` |
| Statuses | `TEXT` + `CHECK`, mapped to Rust enums via `as_str`/`FromStr` |
| Human numbers | Sequences, e.g. `ORD-2026-000123` |
| Case-insensitive email | `CITEXT` |
| Concurrency | `SELECT ... FOR UPDATE` inside workflows; `version` column for optimistic updates |

**Migrations.** `migrations/NNNN_description.sql`, applied by `sqlx::migrate!` (embedded in
the binary). Locally: `DATABASE_RUN_MIGRATIONS=true`. Production: run the image with the
`migrate` argument as a pre-deploy job; keep the flag off in the app. A deployed migration is
never edited — write a new one. Prefer expand/contract for breaking changes (add column →
backfill → switch code → drop later).

**Queries.** Rows are private `#[derive(FromRow)]` structs converted into domain types in the
repository; handlers never see rows. Dynamic filters use `QueryBuilder::push_bind` exclusively.

**Compile-time checking.** The starter uses runtime-checked `sqlx::query_as::<_, Row>(...)` so
`cargo check` works with no database; every query is exercised by the database test suite.
Adopt `sqlx::query_as!` for fixed-shape queries once the team's environment is ready:

1. `cargo install sqlx-cli --no-default-features --features rustls,postgres`
2. convert queries module by module; run `cargo sqlx prepare -- --all-targets` and commit `.sqlx/`
3. set `SQLX_OFFLINE=true` in CI builds; add `cargo sqlx prepare --check` to CI.

Keep `QueryBuilder` for dynamic filters (they cannot be macro-checked).

**Repository pattern.** Repositories are plain async functions taking `impl PgExecutor` or
`&mut PgConnection`, so the same function works on the pool or inside a transaction. Traits
are not added "for testability": domain logic is tested without I/O, and SQL is tested against
real PostgreSQL. Introduce a trait only when there is a second implementation (e.g. a search
index behind `search`).

**Search.** Start with PostgreSQL (`tsvector` columns + GIN indexes, `pg_trgm` for fuzzy
names) behind a `search` module facade. Moving to Meilisearch/OpenSearch then means indexing
via jobs and swapping the facade implementation; callers do not change.

**Transactions.** Example — `quotations::application::accept_quotation`:

```
BEGIN
  SELECT quotation FOR UPDATE; domain: must be SENT and not expired
  UPDATE quotations ... WHERE version = $n        (0 rows → 409)
  INSERT orders, order_items, order_status_history (UNIQUE quotation_id → 409 on duplicate)
  TODO payment schedule, proforma invoice
  INSERT audit_logs
  INSERT background_jobs (QuotationAccepted, idempotency key)
COMMIT
```
Tests assert both the happy path and that an expired quotation writes nothing.

---

## 8. RBAC architecture

```
users ──< user_roles >── roles ──< role_permissions >── permissions
```

* **Roles** (seeded): CUSTOMER, SUPPLIER, SALES_STAFF, PROCUREMENT_STAFF, FINANCE_STAFF,
  LOGISTICS_STAFF, TRAVEL_STAFF, IT_STAFF, BUSINESS_CONSULTANT, MANAGER, ADMIN, SUPER_ADMIN.
* **Permissions** (seeded, mirrored by `shared::auth::Permission`): `quotation.create`,
  `quotation.send`, `order.cancel`, `payment.confirm`, `supplier.verify`, `cms.publish`,
  `report.financial.read`, ...
* **Scope suffix `.own`**: `quotation.read` = all quotations; `quotation.read.own` = only the
  caller's own. `CurrentUser::scope(all, own)` returns `AccessScope::All | Own(user_id)` and the
  repository applies it.
* Code checks **permissions only**, never role names. Changing what a role may do is a data
  change (new migration or admin API), not a code change.

**Request flow**

1. `CurrentUser` extractor reads `Authorization: Bearer`, verifies the JWT (signature, `exp`,
   `iss`).
2. `PrincipalResolver` (implemented by `auth::PgPrincipalResolver`) checks that the session is
   not revoked or expired, that the account is `ACTIVE`, and loads effective permissions.
   Logout or disabling an account therefore takes effect immediately.
3. The use case calls `actor.require(Permission::X)` or `actor.scope(...)`, then applies
   ownership rules (e.g. a customer accepting someone else's quotation gets 404, not 403).

Checks live in the **application layer**, so every entry point (HTTP, jobs, future gRPC) is
protected equally. Planned: cache permissions in Redis keyed by user with invalidation on
role change; admin API for role assignment with audit records.

**Tokens and sessions**: access JWT (15 min, `sub` + `sid`), refresh token = 256-bit random
value stored as SHA-256 hash, rotated on each use, reuse of the previous token revokes the
session. Browser clients receive the refresh token as an `HttpOnly; Secure; SameSite=Strict`
cookie scoped to the refresh path (CSRF-safe because only that endpoint reads cookies); mobile
clients receive it in the body. Email verification and password reset use single-use hashed
tokens in `user_action_tokens`. Passwords: Argon2id, run on the blocking thread pool.

---

## 9. Error handling

Three layers of errors:

| Layer | Type | Example |
|---|---|---|
| Domain / application | One enum per module, `thiserror` | `QuotationError::Expired` |
| API | `shared::error::AppError` | `BusinessRule { code: "QUOTATION_EXPIRED", .. }` |
| Infrastructure | `sqlx::Error`, `anyhow::Error` | converted with `?` into `AppError::Database/Internal` |

Each module implements `From<ModuleError> for AppError` **once**, next to the enum. Handlers
and use cases just use `?`. `AppError::into_response` is the single place that chooses the
status code and body.

Status mapping: validation 422 `VALIDATION_FAILED` (with per-field details); malformed
JSON/query/path 400; unauthenticated 401; missing permission 403; not found 404; conflict /
concurrent modification 409; business rule 422; rate limited 429; database/internal 500.

5xx responses always say `"An unexpected error occurred"` with code `INTERNAL_ERROR`; the full
error is logged with the request ID. SQL text, stack traces, file paths and secrets never
reach clients. Handler panics are caught and returned in the same envelope.

Error codes are part of the public API contract: add new ones freely, never rename existing ones.

`anyhow` is used only at the edges (startup in `main.rs`, job handlers, provider adapters),
never as a domain error type.

---

## 10. API responses

```jsonc
// Single resource                  // Collection
{ "data": { ... } }                  { "data": [ ... ],
                                       "pagination": { "page": 1, "limit": 20,
                                                       "total_items": 57, "total_pages": 3 } }
// Error
{ "error": { "code": "QUOTATION_NOT_FOUND", "message": "quotation was not found",
             "details": null, "request_id": "0199..." } }
```

Conventions:

* Versioned under `/api/v1`; breaking changes go to `/api/v2` for the affected resources.
* `GET` read, `POST` create or trigger an action (`/quotations/{id}/accept`), `PATCH` partial
  update, `DELETE` remove. Actions with business meaning are explicit sub-resources rather than
  generic field updates.
* `201` + `Location` for creation, `204` for empty success, `202` for accepted async work.
* JSON field names `snake_case`; enums `SCREAMING_SNAKE_CASE`; timestamps RFC 3339 UTC;
  money `{ "amount": "1250.50", "currency": "USD" }` with the amount as a string.
* Pagination: `?page=&limit=` (max 100). Use cursor pagination (`?cursor=&limit=`, ordered by
  `(created_at, id)`) for feeds such as notifications, audit logs and tracking events.
* Filters are typed query structs (`ListRequirementsQuery`): unknown enum values → 400, invalid
  shapes → 422.
* `Idempotency-Key` header is required on money-moving POSTs (planned for payments).
* Every response carries `x-request-id`; clients should log it with errors.
* Response DTOs are explicit structs. Domain and database types are never serialised directly
  (`UserResponse` has no password hash by construction).

---

## 11. Background jobs and events

**Queue.** `background_jobs` table + `jobs::worker`. A use case enqueues with the same
transaction it commits (`jobs::enqueue(&mut *tx, &Job::..., options)`), so jobs exist exactly
when the business change exists (transactional outbox). Workers claim batches with
`FOR UPDATE SKIP LOCKED`, so any number of instances can run the worker safely.

* Retries with exponential backoff (30 s → 1 h), then `DEAD` for inspection.
* Stale `RUNNING` jobs (crashed worker) are released after 15 minutes.
* `idempotency_key` de-duplicates enqueues; **handlers must be idempotent** because a job can
  run more than once.
* Graceful shutdown: the worker stops claiming when the cancellation token fires and finishes
  the current job; `main` awaits it before closing the pool.

**Events.** Application events are modelled as job variants (`QuotationAccepted`, planned:
`OrderCreated`, `PaymentReceived`, `OrderShipped`, `SupplierVerified`). `jobs/handlers.rs`
dispatches each to the owning modules (notifications, PDF generation, integrations). This
keeps upstream modules free of imports from downstream ones and needs no broker. If fan-out
grows, add an `events` table with per-subscriber cursors; move to a broker only when a
separate service must consume them.

**Why not Redis for the queue?** A Redis enqueue cannot be part of the PostgreSQL transaction,
which reintroduces lost or phantom jobs. Redis remains for rate limiting, brute-force
counters and caches. The queue is only reachable through `jobs::enqueue` and `jobs::worker`, so
it can be replaced (e.g. by a Redis Streams or cloud queue relay reading the outbox) without
touching modules.

Planned job kinds: send email / SMS / WhatsApp, generate invoice & quotation PDFs, resize
images, scan and validate uploaded documents, synchronise shipment tracking, generate reports,
expire quotations (scheduled), clean up expired sessions and tokens (scheduled).

---

## 12. File storage

* Binaries live in S3-compatible storage. PostgreSQL stores only metadata:
  owner, entity link, document type, storage key, original filename, MIME type, size, checksum,
  status (`PENDING_UPLOAD → SCANNING → AVAILABLE | REJECTED`), visibility, timestamps.
* `shared::storage::ObjectStorage` is the only interface: `put_object`, `delete_object`,
  `presigned_upload_url`, `presigned_download_url`. The planned implementation is
  `S3ObjectStorage` (`aws-sdk-s3` with custom endpoint), added to `AppState` with the documents
  module.
* Upload flow: client asks `documents` for an upload slot (permission + declared type/size
  checked) → receives a short-lived presigned PUT URL for a server-generated key → uploads
  directly → calls complete → job verifies size, magic-byte MIME type and checksum, optionally
  virus-scans, then marks `AVAILABLE`.
* Download flow: permission check in `documents` → short-lived presigned GET URL (minutes).
  Buckets are private; no public object URLs.
* Keys never contain user-supplied filenames:
  `customers/{customer_id}/documents/{document_id}`.
* Limits: per-type allow-lists (PDF, JPEG, PNG; passports/visas PDF/JPEG only), maximum sizes per
  document type, request body limit for JSON endpoints stays small (`APP_REQUEST_BODY_LIMIT`).
* Generated files (invoices, quotation PDFs) are written by jobs with `put_object`.

---

## 13. Logging, observability and auditing

**Logging** (`tracing`):

* Every request gets a UUID v7 `x-request-id` (or keeps the one set by the load balancer),
  propagated to the response, recorded on the `http_request` span with method, path, status
  and latency (ms).
* JSON logs in staging/production (`LOG_FORMAT=json`), pretty logs locally; level via
  `LOG_LEVEL` (`info,sqlx=warn`).
* `Authorization` and `Cookie` headers are marked sensitive; query strings are not logged.
* `println!`, `dbg!`, `unwrap()` are denied by clippy configuration.
* Planned: OpenTelemetry exporter (traces + metrics) via `tracing-opentelemetry`; the
  request ID doubles as the correlation ID in the meantime.

**Auditing** (`modules::audit`):

* `audit::record(&mut *tx, AuditEntry::new(Actor::User(id), "quotation.accepted", "quotation", id))`
  inside the business transaction, with the request ID captured automatically.
* Action naming: `<module>.<past_tense>` — `auth.login_succeeded`, `quotation.accepted`,
  `order.status_changed`, `payment.confirmed`, `supplier.verification_changed`,
  `role.granted`, `settings.updated`.
* `audit_logs` is append-only (trigger rejects UPDATE/DELETE). Metadata must never contain
  passwords, tokens, card data or document contents.
* Must be audited: logins (success and failure), session revocation, quotation create/update/
  send/accept/reject, order status changes, payment record/confirm/refund, supplier
  verification, role and permission changes, admin actions, settings changes, document
  deletion.

---

## 14. Testing

| Level | Location | Infrastructure | Examples |
|---|---|---|---|
| Unit | `#[cfg(test)]` next to code | none | status transition tables, quotation totals and expiry, money rounding, pagination clamping, JWT round trip, Argon2, permission scopes, config secret redaction |
| API (no infra) | `tests/api/health_and_errors.rs` | none (lazy pool, lazy Redis) | error envelope, 401/422 paths, security headers |
| API + database | `tests/api/*.rs`, `#[sqlx::test]`, `#[ignore]` | PostgreSQL test server | quotation acceptance atomicity and idempotency, ownership 404, order transitions, cancellation permission, requirement scoping and filters, session revocation, supplier verification, job worker |

`#[sqlx::test]` creates a fresh database per test from `DATABASE_URL`, applies all migrations,
and drops it afterwards — tests are isolated and can run in parallel. **Never point
`DATABASE_URL` at a shared or production server when running tests.**

```bash
cargo test                                          # fast, no infrastructure
DATABASE_URL=postgres://miracle:miracle@localhost:5432/miracle_test \
  cargo test -- --include-ignored                   # everything (what CI runs)
```

Test helpers in `tests/api/common` create users with roles, sessions and access tokens,
customers and quotations directly in SQL, then drive the real router with
`tower::ServiceExt::oneshot`.

Next tests to add as features land: register → verify email → login → refresh rotation and
reuse detection; brute-force lockout; payment recording with idempotency key; webhook
duplicate delivery; document permission checks and presigned URL issuance.

---

## 15. Configuration

Typed parsing in `config/mod.rs`; invalid critical configuration aborts startup with a
message naming the variable. Full list with defaults: [`.env.example`](../.env.example).

| Group | Variables |
|---|---|
| App | `APP_ENV` (development/test/staging/production), `APP_HOST`, `APP_PORT`, `APP_REQUEST_BODY_LIMIT`, `APP_REQUEST_TIMEOUT`, `APP_API_DOCS_ENABLED` |
| Logging | `LOG_FORMAT`, `LOG_LEVEL` |
| Database | `DATABASE_URL`, `DATABASE_MAX_CONNECTIONS`, `DATABASE_MIN_CONNECTIONS`, `DATABASE_ACQUIRE_TIMEOUT`, `DATABASE_RUN_MIGRATIONS` |
| Redis | `REDIS_URL` |
| Auth | `JWT_SECRET` (≥ 32 bytes), `JWT_ISSUER`, `JWT_ACCESS_TTL`, `JWT_REFRESH_TTL` (seconds) |
| Storage | `S3_ENDPOINT`, `S3_BUCKET`, `S3_REGION`, `S3_ACCESS_KEY`, `S3_SECRET_KEY` (optional until documents ship) |
| CORS | `FRONTEND_URL` (comma-separated origins; `*` rejected in staging/production) |
| Jobs | `JOBS_ENABLED`, `JOBS_POLL_INTERVAL_MS`, `JOBS_BATCH_SIZE` |

Environment policy: Swagger UI defaults to on only in development; HSTS is sent in
staging/production; placeholder secrets are rejected outside development. In deployed
environments secrets come from the platform secret manager, never from files in the image.

---

## 16. Cargo dependencies

| Crate | Purpose | Notes |
|---|---|---|
| `axum` 0.8 | HTTP routing, extractors | `macros` feature |
| `tokio` 1 | Runtime, signals, timers | only needed features |
| `tower` 0.5, `tower-http` 0.7 | Middleware: trace, request-id, CORS, timeout, catch-panic, headers | |
| `serde`, `serde_json` | Serialization | |
| `validator` 0.21 | DTO validation | `derive` |
| `sqlx` 0.9 | PostgreSQL, migrations, test databases | `postgres, runtime-tokio, tls-rustls, uuid, chrono, rust_decimal, json, migrate, macros` |
| `redis` 1 | Rate limiting, caches, readiness | `tokio-comp, connection-manager` |
| `uuid` 1 | IDs | `v7, serde` |
| `chrono` 0.4 | Timestamps | `serde`, no default features |
| `rust_decimal` 1 | Money | `serde-with-str` |
| `argon2` 0.6 | Password hashing | Argon2id defaults |
| `jsonwebtoken` 11 | Access tokens | `rust_crypto` backend |
| `sha2`, `base64`, `getrandom` | Opaque token generation and hashing | |
| `thiserror` 2 / `anyhow` 1 | Typed errors / edge errors | |
| `tracing`, `tracing-subscriber` | Structured logging | `json, env-filter` |
| `utoipa` 5, `utoipa-swagger-ui` 9 | OpenAPI + UI | `vendored` UI assets (no download at build time) |
| `async-trait` | Object-safe async traits for provider abstractions | |
| `tokio-util` | `CancellationToken` for graceful shutdown | |
| `dotenvy` | `.env` in local development | |

Add later, with the feature that needs them: `aws-sdk-s3` (documents), `lettre` or provider
SDK (email), `reqwest` (gateway/carrier integrations), `tracing-opentelemetry` +
`opentelemetry-otlp` (observability), `cargo-deny` / `cargo-audit` in CI (supply chain).

---

## 17. Security checklist

| Concern | Status |
|---|---|
| Password hashing | ✅ Argon2id on blocking pool |
| Authentication | ✅ JWT verification + session check per request; login/refresh flows TODO |
| Authorization | ✅ permission checks in use cases, `.own` scoping, ownership → 404 |
| Session revocation | ✅ logout, disabled accounts rejected immediately |
| SQL injection | ✅ bind parameters only, LIKE escaping |
| Secure headers | ✅ nosniff, frame DENY, no-referrer, CSP + no-store on API, HSTS when deployed |
| CORS | ✅ explicit origins; wildcard rejected when deployed |
| Request size / timeouts | ✅ body limit, per-request timeout |
| Error leakage | ✅ opaque 5xx, request ID, panic capture |
| Secrets | ✅ `Secret` redaction, sensitive headers, `.env` ignored by git |
| Idempotency | ✅ unique constraints for order-per-quotation, webhook events, payments, jobs |
| Audit logging | ✅ append-only table, transactional writes |
| Rate limiting / brute force | ◻️ Redis limiter on auth routes; account lockout columns exist |
| Upload validation | ◻️ documents module (magic bytes, size, allow-lists, scanning) |
| CSRF | ◻️ applies to the refresh cookie endpoint only (SameSite=Strict + path scope + origin check) |
| Dependency audit | ◻️ `cargo deny check` in CI |

---

## 18. Extracting a module into a service

The boundaries above make extraction mechanical when scale requires it (likely candidates:
notifications, search, logistics integrations, reports):

1. The module already has a facade (`mod.rs`) — turn those functions into an HTTP/gRPC client
   with the same signatures.
2. It already owns its tables — move them to the new service's database.
3. Cross-module transactions become job/event hand-offs: the outbox already delivers
   `Job` variants at-least-once, and handlers are idempotent.
4. `CurrentUser` and permissions travel as a verified token; the service reuses
   `shared::auth` (publish `shared` as an internal crate first by converting the repository
   into a Cargo workspace).
