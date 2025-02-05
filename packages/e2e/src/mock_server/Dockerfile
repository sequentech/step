# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM rust:1.82.0

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8500

RUN apt-get update && apt-get install -y chromium protobuf-compiler libprotobuf-dev
RUN rustup default nightly

WORKDIR /app/e2e/src/mock_server
RUN cargo build

CMD ["cargo", "run"]