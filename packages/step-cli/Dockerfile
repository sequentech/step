# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM rust:1.96.0-slim-bookworm as builder

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

COPY . .

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
