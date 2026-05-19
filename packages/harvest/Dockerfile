# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM rust:1.90.0-slim-bookworm

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8400

WORKDIR /app
# copy entire packages directory (monorepo)
WORKDIR /app
COPY . .

RUN apt-get update && apt-get install -y chromium protobuf-compiler libprotobuf-dev

WORKDIR /app/harvest
RUN cargo build

CMD ["cargo", "run"]
