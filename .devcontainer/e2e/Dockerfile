# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
FROM rust:1.96.0-bookworm AS rust
FROM grafana/k6:1.6.0 AS k6
FROM mcr.microsoft.com/playwright:v1.62.1-noble
COPY --from=rust /usr/local/cargo /usr/local/cargo
COPY --from=rust /usr/local/rustup /usr/local/rustup
COPY --from=k6 /usr/bin/k6 /usr/local/bin/k6
ENV PATH=/usr/local/cargo/bin:$PATH CARGO_HOME=/usr/local/cargo RUSTUP_HOME=/usr/local/rustup \
    CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config libssl-dev protobuf-compiler libprotobuf-dev m4 \
    cmake clang libclang-dev python3-yaml python3-requests postgresql-client \
    openjdk-17-jre-headless ca-certificates curl git && rm -rf /var/lib/apt/lists/*
RUN rustup component add llvm-tools-preview rustfmt && rustup target add wasm32-unknown-unknown \
    && cargo install cargo-llvm-cov --version 0.9.1 --locked \
    && corepack enable && corepack prepare yarn@1.22.22 --activate
RUN curl -fsSL https://github.com/rustwasm/wasm-pack/releases/download/v0.13.1/wasm-pack-v0.13.1-x86_64-unknown-linux-musl.tar.gz \
    | tar -xz --strip-components=1 -C /usr/local/bin --wildcards '*/wasm-pack' \
    && curl -fsSL https://github.com/wasm-bindgen/wasm-bindgen/releases/download/0.2.128/wasm-bindgen-0.2.128-x86_64-unknown-linux-musl.tar.gz \
    | tar -xz --strip-components=1 -C /usr/local/bin --wildcards '*/wasm-bindgen' '*/wasm-bindgen-test-runner' \
    && curl -fsSL https://github.com/hasura/graphql-engine/releases/download/v2.33.1/cli-hasura-linux-amd64 -o /usr/local/bin/hasura \
    && chmod +x /usr/local/bin/hasura
WORKDIR /workspaces/step
CMD ["sleep", "infinity"]
