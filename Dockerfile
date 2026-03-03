# syntax=docker/dockerfile:1.7

FROM rust:1.93.1-slim-trixie AS builder
WORKDIR /app

# Build dependency layer first for better cache reuse.
COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src

RUN cargo build --release --locked --bin mtr

FROM debian:trixie-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 appuser \
    && useradd --system --uid 10001 --gid appuser --home /nonexistent --shell /usr/sbin/nologin appuser

COPY --from=builder /app/target/release/mtr /usr/local/bin/windows-mtr
RUN ln -s /usr/local/bin/windows-mtr /usr/local/bin/mtr

USER appuser
ENTRYPOINT ["/usr/local/bin/windows-mtr"]
