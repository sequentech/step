# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM sequentech.local/offline-dependencies:latest AS deps

FROM sequentech.local/ci-builder:latest AS builder

WORKDIR /app
# copy entire packages directory (monorepo)
COPY . .

# Copy vendored dependencies and config
COPY --from=deps /vendor-cache /vendor-cache
RUN mkdir -p .cargo && cat /vendor-cache/config.toml >> .cargo/config.toml

RUN cp /app/windmill/external-bin/ecies-tool.jar /usr/local/bin/

# java -jar ecies-tool.jar create-keys public.pem private.pem

WORKDIR /app/windmill
RUN cargo build

CMD ["cargo", "run", "--bin", "main", "consume", "-q", "short_queue", "tally_queue", "beat", "reports_queue" ]