# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

ARG RUST_IMAGE=rust:1.96.0-bookworm
ARG K6_IMAGE=grafana/k6:1.6.0
ARG WORKER_IMAGE=debian:bookworm-slim
ARG BROWSER_IMAGE=node:22-bookworm-slim
FROM ${K6_IMAGE} AS k6-binary
FROM ${RUST_IMAGE} AS worker-build
WORKDIR /build/packages/step-cli
COPY packages/voting-load/worker.Cargo.toml Cargo.toml
COPY packages/voting-load/worker.Cargo.lock Cargo.lock
COPY packages/voting-load/worker.rs src/main.rs
COPY packages/step-cli/src/load/config.rs packages/step-cli/src/load/files.rs packages/step-cli/src/load/input.rs packages/step-cli/src/load/worker.rs src/load/
COPY packages/voting-portal/src/queries/GetVoterStatus.ts packages/voting-portal/src/queries/InsertCastVote.ts /build/packages/voting-portal/src/queries/
RUN cargo build --release --locked

# Protocol workers contain neither Python, Node.js nor administrator CLI clients.
FROM ${WORKER_IMAGE} AS k6
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=k6-binary /usr/bin/k6 /usr/local/bin/k6
COPY --from=worker-build /build/packages/step-cli/target/release/step-load-worker /usr/local/bin/step-load-worker
WORKDIR /runner
COPY packages/voting-load/*.js /runner/packages/voting-load/
ENV HOME=/tmp STEP_LOAD_CONTAINER=1
USER 1000:1000
ENTRYPOINT ["/usr/local/bin/step-load-worker"]

FROM ${BROWSER_IMAGE} AS chromium
ARG PLAYWRIGHT_VERSION=1.62.1
WORKDIR /runner
ENV PLAYWRIGHT_BROWSERS_PATH=/ms-playwright
RUN npm install --save-exact @playwright/test@${PLAYWRIGHT_VERSION} && npx playwright install --with-deps chromium
COPY --from=worker-build /build/packages/step-cli/target/release/step-load-worker /usr/local/bin/step-load-worker
COPY packages/voting-portal/playwright.scale.config.ts /runner/packages/voting-portal/
COPY packages/voting-portal/test/load/flow.ts packages/voting-portal/test/load/scale.spec.ts /runner/packages/voting-portal/test/load/
ENV HOME=/tmp STEP_LOAD_CONTAINER=1
USER 1000:1000
ENTRYPOINT ["/usr/local/bin/step-load-worker"]
