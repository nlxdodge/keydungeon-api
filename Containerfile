# syntax=docker/dockerfile:1

# ---------- Build stage ----------
# `init.sql` is embedded into the binary with include_str!, so it only needs to
# exist at build time, not in the runtime image.
FROM rust:1.99.1-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY init.sql ./init.sql
COPY src ./src

# Caches keep the registry and incremental build artifacts between builds
# without baking them into the final image.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked && \
    cp target/release/keydungeon-api /app/keydungeon-api && \
    strip /app/keydungeon-api

# Prepare the data directory with the UID/GID used by the distroless nonroot user.
RUN mkdir -p /data/tmp && chown -R 65532:65532 /data

# ---------- Runtime stage ----------
# Distroless = a container image with no shell, package manager, or toolchain.
# `cc` provides exactly the glibc/libgcc runtime a dynamically linked Rust
# binary needs. Nothing else is in the image besides the app itself.
FROM gcr.io/distroless/cc-debian12:nonroot
# This tag runs as UID/GID 65532. If you ever need to run as root instead,
# use gcr.io/distroless/cc-debian12 and drop the `USER` line below.

WORKDIR /app
COPY --from=builder --chown=65532:65532 /data /data
COPY --from=builder /app/keydungeon-api /app/keydungeon-api

ENV DATABASE_URL=sqlite:///data/keydungeon.db \
    TMPDIR=/data/tmp

EXPOSE 4321
USER nonroot:nonroot
ENTRYPOINT ["/app/keydungeon-api"]
