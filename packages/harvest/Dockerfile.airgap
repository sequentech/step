# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM sequentech.local/offline-dependencies:latest AS deps

FROM sequentech.local/ci-builder:latest

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8400

WORKDIR /app
# copy entire packages directory (monorepo)
COPY . .

# Copy vendored dependencies and config
COPY --from=deps /vendor-cache /vendor-cache
RUN mkdir -p .cargo && cat /vendor-cache/config.toml >> .cargo/config.toml
WORKDIR /app/harvest
RUN cargo build

CMD ["cargo", "run"]
