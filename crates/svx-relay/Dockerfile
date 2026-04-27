FROM rust:1.85-slim AS builder
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates/svx-core/Cargo.toml crates/svx-core/Cargo.toml
COPY crates/svx-relay/Cargo.toml crates/svx-relay/Cargo.toml
COPY crates/svx-cli/Cargo.toml crates/svx-cli/Cargo.toml
RUN mkdir -p crates/svx-core/src crates/svx-relay/src crates/svx-cli/src/bin && \
    echo 'pub fn _stub() {}' > crates/svx-core/src/lib.rs && \
    echo 'fn main() {}' > crates/svx-relay/src/main.rs && \
    echo 'fn main() {}' > crates/svx-cli/src/main.rs && \
    echo 'fn main() {}' > crates/svx-cli/src/bin/gen-test-identity.rs && \
    cargo build --release --bin svx-relay && \
    rm -rf crates

COPY crates ./crates
RUN touch crates/svx-core/src/lib.rs crates/svx-relay/src/main.rs && \
    cargo build --release --bin svx-relay

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/svx-relay /usr/local/bin/svx-relay
ENV PORT=8080 RUST_LOG=svx_relay=info,tower_http=info
EXPOSE 8080
USER 1000:1000
ENTRYPOINT ["/usr/local/bin/svx-relay"]
