# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

FROM grafana/k6:1.6.0 AS k6-binary

# The default protocol worker contains no browser, Node.js or encryption runtime.
FROM python:3.13-slim-bookworm AS k6
COPY --from=k6-binary /usr/bin/k6 /usr/local/bin/k6
WORKDIR /runner
COPY scripts/voting_e2e /runner/scripts/voting_e2e
ENV HOME=/tmp
USER 1000:1000
ENTRYPOINT ["python3", "/runner/scripts/voting_e2e/scale.py"]

# Build this target only for full UI journeys; version matches the test runner.
FROM node:22-bookworm-slim AS chromium
RUN apt-get update && apt-get install -y --no-install-recommends python3 ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /runner
RUN npm install --save-exact @playwright/test@1.62.1 && npx playwright install --with-deps chromium
COPY --from=k6-binary /usr/bin/k6 /usr/local/bin/k6
COPY scripts/voting_e2e /runner/scripts/voting_e2e
COPY packages/voting-portal/playwright.scale.config.ts /runner/packages/voting-portal/
COPY packages/voting-portal/test/load/flow.ts packages/voting-portal/test/load/scale.spec.ts /runner/packages/voting-portal/test/load/
ENV HOME=/tmp
USER 1000:1000
ENTRYPOINT ["python3", "/runner/scripts/voting_e2e/scale.py"]
