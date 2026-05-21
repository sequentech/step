# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM sequentech.local/ci-builder:latest

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8400

WORKDIR /app
# copy entire packages directory (monorepo)
WORKDIR /app
COPY . .


WORKDIR /app/harvest
RUN cargo build

CMD ["cargo", "run"]
