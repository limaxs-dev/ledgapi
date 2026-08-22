# syntax=docker/dockerfile:1.7
FROM rust:1.97-bookworm AS builder
WORKDIR /app

RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Pre-warm dependency cache using a fake main so source changes don't invalidate it.
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY templates ./templates
RUN mkdir -p src && echo "fn main() {}" > src/main.rs \
 && cargo build --release --locked || true

# Real source.
COPY src ./src
RUN touch src/main.rs && cargo build --release --locked --bin ledgapi \
 && strip target/release/ledgapi

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates wget tini \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --create-home --shell /usr/sbin/nologin app

COPY --from=builder /app/target/release/ledgapi /usr/local/bin/ledgapi
COPY migrations  /app/migrations
COPY templates   /app/templates
COPY docker/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh \
 && mkdir -p /data && chown app:app /data

USER app
ENV APP__SERVER__BIND=0.0.0.0:8080
EXPOSE 8080
VOLUME ["/data"]
HEALTHCHECK --interval=15s --timeout=2s --start-period=40s --retries=3 \
    CMD wget -qO- http://127.0.0.1:8080/healthz || exit 1

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/entrypoint.sh"]
CMD ["ledgapi"]
