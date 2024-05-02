# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM rust:1.70.0

WORKDIR /app
COPY ./immu-board ./immu-board
COPY ./immudb-rs ./immudb-rs
COPY ./strand ./strand
COPY ./board-messages ./board-messages
COPY ./sequent-core ./sequent-core
COPY ./windmill ./windmill
COPY ./wrap-map-err ./wrap-map-err
COPY ./velvet ./velvet

RUN apt-get update && apt-get install -y chromium protobuf-compiler libprotobuf-dev
RUN rustup default nightly

WORKDIR /app/windmill
RUN cargo build

CMD ["cargo", "run", "--bin", "main", "consume", "-q", "short_queue", "tally_queue", "beat", "reports_queue", "beat" ]