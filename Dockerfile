# syntax=docker/dockerfile:1.7

# ---- Build -------------------------------------------------------------------
FROM rust:1.98-slim-trixie AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

# Cache mounts keep the registry and incremental artifacts between builds.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked \
    && cp target/release/miracle_backend /usr/local/bin/miracle_backend

# ---- Runtime -----------------------------------------------------------------
# Distroless: no shell or package manager; runs as a non-root user.
FROM gcr.io/distroless/cc-debian13:nonroot

COPY --from=builder /usr/local/bin/miracle_backend /usr/local/bin/miracle_backend

ENV APP_ENV=production \
    APP_HOST=0.0.0.0 \
    APP_PORT=8080 \
    LOG_FORMAT=json

EXPOSE 8080
USER nonroot

# Probes: GET /health/live (liveness) and GET /health/ready (readiness).
# Migrations: run the same image with the `migrate` argument as a pre-deploy job.
ENTRYPOINT ["/usr/local/bin/miracle_backend"]
