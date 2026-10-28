FROM rust:1.97.0-bookworm AS build
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /build
# build.py supplies only these allowlisted inputs and a minimal inheritance workspace.
COPY Cargo.toml ./Cargo.toml
COPY packages/domain/Cargo.toml ./packages/domain/Cargo.toml
COPY packages/domain/src ./packages/domain/src
COPY apps/cloud-service/Cargo.toml apps/cloud-service/Cargo.lock ./apps/cloud-service/
COPY apps/cloud-service/src ./apps/cloud-service/src
COPY apps/cloud-service/migrations ./apps/cloud-service/migrations
RUN cargo build --release --locked --manifest-path apps/cloud-service/Cargo.toml

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /opt/superfactory/bin /opt/superfactory/config
COPY --from=build /build/apps/cloud-service/target/release/sf-cloud-service /opt/superfactory/bin/sf-cloud-service
ENV PORT=8080 SF_CLOUD_LISTEN_ADDRESS=0.0.0.0 \
    SF_CLOUD_BINDINGS_FILE=/opt/superfactory/config/demo-bindings.json
USER 10001:10001
EXPOSE 8080
STOPSIGNAL SIGTERM
HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
    CMD test "$(curl --silent --output /dev/null --write-out '%{http_code}' --max-time 4 http://127.0.0.1:${PORT}/healthz)" = 204
ENTRYPOINT ["/opt/superfactory/bin/sf-cloud-service"]
CMD ["serve"]
