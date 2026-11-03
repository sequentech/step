# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
FROM rust:1.96.0-slim-bookworm@sha256:4732ca96fd086cb9be682050c3f0176288eebaac2b80aa2bcefccfaf198e1950 AS builder
WORKDIR /app
COPY . .
RUN cargo build --locked --release -p b4 --features postgres --bin b4v6

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/b4v6 /usr/local/bin/b4v6
USER 65532:65532
ENV WBRAID_B4_BIND=0.0.0.0:3005
EXPOSE 3005
ENTRYPOINT ["/usr/local/bin/b4v6"]
