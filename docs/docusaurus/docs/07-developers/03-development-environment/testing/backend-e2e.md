---
title: Backend E2E journeys
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The backend journeys use real PostgreSQL, Hasura, Keycloak, RabbitMQ, ImmuDB,
MinIO, B4 and two trustees. They exercise administrator commands and the voting
portal's GraphQL protocol, including ballot encryption and tallying.

From the repository root, with Docker Engine and Compose 2.24.4 or newer:

```bash
scripts/e2e/run.sh
# If Docker requires sudo:
DOCKER="sudo docker" scripts/e2e/run.sh
```

The first run builds the service images and Rust binaries in the same container
image that executes them. Allow at least 8 GB RAM and 40 GB free disk space; use
`CARGO_BUILD_JOBS=1` on smaller machines. No host Rust, Node or Python installation
is needed. The runner uses development credentials from `.env.development` and
creates a fresh `step-e2e` Compose project without publishing host ports. It
removes that project's containers and volumes when finished.

For iteration:

```bash
scripts/e2e/run.sh --skip-images --skip-build --keep
scripts/e2e/run.sh --skip-images --skip-build -k automatic_key_ceremony
scripts/e2e/run.sh --down
```

`-k` runs through the last matching test, including its prerequisites. Skipping
images or binaries is appropriate only while their sources are unchanged.
`--keep` retains the stack for diagnosis; the next run still starts fresh.
`STEP_E2E_PROJECT` selects another isolated project name. Set `STEP_E2E_PORTS=1`
to expose Hasura, Keycloak and MinIO on dynamically allocated loopback ports;
`docker ps` shows the allocated ports.

Results are written to `.cache/backend-e2e/run/journeys.json`, with CLI output and
service logs beside it. Override this directory with `STEP_E2E_OUTPUT_DIR`.
The `Backend E2E journeys` workflow runs the same command on affected pull
requests and uploads those artifacts, including after failures.

The ordered tests in `scripts/e2e/journeys/test_journeys.py` cover tenant bootstrap,
event import and invalid bundles, voter import, automatic key generation,
private ballot publication, cast/revote/closing rules and results matching the
last accepted ballot. Fixtures live in `fixtures.py`; clients use Python's
standard library. Add assertions against observable responses and persisted
results, with valid controls for rejection cases.

Known defects have explicit expected-failure probes: only the documented failure
is accepted, and a passing probe fails the run so its marker must be removed.
Other errors and skipped journeys fail the run. These probes do not replace the
seven required passing journeys.
