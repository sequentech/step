# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM rust:1.82.0-slim-bookworm as builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    chromium \
    protobuf-compiler \
    libprotobuf-dev \
    m4 \
    make \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install the specific nightly toolchain
RUN rustup toolchain install nightly-2024-09-01 \
    && rustup default nightly-2024-09-01
COPY ./braid ./braid
COPY ./b3 ./b3
COPY ./electoral-log ./electoral-log
COPY ./harvest ./harvest
COPY ./strand ./strand
COPY ./immu-board ./immu-board
COPY ./immudb-rs ./immudb-rs
COPY ./sequent-core ./sequent-core
COPY ./velvet ./velvet
COPY ./windmill ./windmill
COPY ./e2e ./e2e
COPY ./step-cli ./step-cli
COPY ./orare ./orare
COPY ./wrap-map-err ./wrap-map-err
COPY ./braid/scripts ./braid/scripts
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./.cargo ./.cargo

WORKDIR /app/step-cli
RUN cargo build --release --locked

FROM debian:bookworm
# Install any necessary runtime dependencies for production
RUN echo "deb http://deb.debian.org/debian/ bookworm-proposed-updates main" >> /etc/apt/sources.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends libssl3 chromium ca-certificates wget software-properties-common \
    && rm -rf /var/lib/apt/lists/*
RUN update-ca-certificates
# Copy the compiled binaries from the builder stage (beat + windmill:main)
COPY --from=builder /app/target/release/step-cli /usr/local/bin/step-cli
WORKDIR /usr/local/bin
CMD ["/usr/local/bin/step-cli"]
