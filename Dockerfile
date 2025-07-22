FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/hourly-wolves /usr/local/bin
ENTRYPOINT ["/usr/local/bin/hourly-wolves"]

ARG VERSION
ARG VCS_REF
ARG BUILD_DATE

LABEL org.opencontainers.image.title="hourly-wolves" \
    org.opencontainers.image.description="Silly webhook-based wolfposting for Discord." \
    org.opencontainers.image.url="https://github.com/kaylendog/hourly-wolves" \
    org.opencontainers.image.source="https://github.com/kaylendog/hourly-wolves" \
    org.opencontainers.image.version="${VERSION}" \
    org.opencontainers.image.created="${BUILD_DATE}" \
    org.opencontainers.image.revision="${VCS_REF}" \
    org.opencontainers.image.licenses="MIT" \
    org.opencontainers.image.authors="Skye Elliot <actuallyori@gmail.com>"
