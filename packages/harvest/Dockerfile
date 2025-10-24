# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM rust:1.90.0-slim-bookworm

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8400

WORKDIR /app
COPY ./harvest ./harvest
COPY ./immu-board ./immu-board
COPY ./immudb-rs ./immudb-rs
COPY ./strand ./strand
COPY ./board-messages ./board-messages
COPY ./sequent-core ./sequent-core
COPY ./windmill ./windmill
COPY ./wrap-map-err ./wrap-map-err
COPY ./velvet ./velvet

RUN apt-get update && apt-get install -y chromium protobuf-compiler libprotobuf-dev

WORKDIR /app/harvest
RUN cargo build

CMD ["cargo", "run"]
