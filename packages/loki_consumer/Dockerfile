# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM rust:1.82.0

# Required env variables
# LOKI_URL=http://loki:3100
# LOKI_QUERY='{container="keycloak"} |= `- phoneNumber=` or `- message=`'
# LOKI_TAIL_LIMIT=100
# RECONNECT_DELAY_SECS=5

WORKDIR /app
COPY . ./loki_consumer

RUN apt-get update && apt-get install -y chromium protobuf-compiler libprotobuf-dev
RUN rustup default nightly

WORKDIR /app/loki_consumer
RUN cargo build

CMD ["cargo", "run", "--release"]
