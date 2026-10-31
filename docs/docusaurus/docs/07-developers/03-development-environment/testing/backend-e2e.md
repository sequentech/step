---
title: Backend E2E journeys
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The backend journeys use real PostgreSQL (with the electoral log's databases
and the task queues), Hasura, Keycloak, MinIO, B4 and two trustees. They exercise administrator commands and the voting
portal's GraphQL protocol, including ballot encryption and tallying.

From the repository root, with Python 3, `flock`, Docker Engine and Compose 2.24.4 or newer:

```bash
scripts/e2e/run.sh
# If Docker requires sudo:
DOCKER="sudo docker" scripts/e2e/run.sh
```

The first run builds the service images and Rust binaries in the same container
image that executes them. Allow at least 8 GB RAM and 40 GB free disk space; use
`CARGO_BUILD_JOBS=1` on smaller machines. No host Rust or Node installation is needed. Python 3 holds the project lock. The runner uses development credentials from `.env.development` and
creates a uniquely named `step-e2e-...` Compose project without publishing host ports. It
removes that project's containers and volumes when finished.

The isolated bootstrap enables the tenant's `api-key-client` and assigns
`API_KEY_CLIENT_SECRET` from the test environment before authenticating the
administrator and trustees. Tenant creation rotates client secrets; the realm
template's defaults do not provide credentials for the generated realm.

For iteration:

```bash
STEP_E2E_PROJECT=my-e2e scripts/e2e/run.sh --skip-images --skip-build --keep
scripts/e2e/run.sh --skip-images --skip-build -k automatic_key_ceremony
STEP_E2E_PROJECT=my-e2e scripts/e2e/run.sh --down
```

`-k` runs through the last matching test, including its prerequisites. Skipping
images or binaries is appropriate only while their sources are unchanged.
`--keep` retains the stack for diagnosis. To reuse an explicit project name, first
remove that retained stack with `STEP_E2E_PROJECT=<name> scripts/e2e/run.sh --down`;
a default invocation chooses a new name automatically.
`STEP_E2E_PROJECT` selects an explicit project name. Starting a project that already
has Compose containers, networks or volumes fails without deleting them. `--down`
requires an explicit project name; use the name printed at startup to remove a
retained default run. Set `STEP_E2E_PORTS=1`
to expose Hasura, Keycloak and MinIO on dynamically allocated loopback ports;
`docker ps` shows the allocated ports.
`STEP_E2E_COMPOSE_OVERLAY=/absolute/path/manual.yml` adds an existing readable
Compose file after the standard, port and coverage overlays. Relative paths are
resolved from the caller's directory. Use the same override for startup and
`--down`; it can give an isolated stack its own service image tags and browser
URLs without editing the development Compose files. `--bootstrap-only --keep`
prepares services and the administrator without executing voting journeys.

Results are written to `.cache/backend-e2e/<project>/journeys.json`, with CLI output and
service logs beside it. Override this directory with `STEP_E2E_OUTPUT_DIR`.
The Compose overlays also work with `.env.development` alone: binaries default
to `.cache/backend-e2e/bin` and output to `.cache/backend-e2e/default` within
the checkout. The runner writes its per-run paths to `compose.env` and passes
that file to Compose, so Compose does not require shell exports for these paths.
The `Backend E2E journeys` workflow runs the same journeys on affected pull
requests and uploads those artifacts, including after failures. It splits the
build across two runners: one job builds the release binaries (`trustee`,
`b4v6`) with `STEP_E2E_BINARIES=release scripts/e2e/run.sh --build-only`, while
the journeys job builds the images and the debug binaries
(`STEP_E2E_BINARIES=debug`), takes the release pair from the Actions cache and
runs with `--skip-images --skip-build`. `--build-only` builds what
`STEP_E2E_IMAGES` names (default: every image the stack needs) and the
selected binaries, then stops. Pushes to `main`, `ovcs` and release branches
seed the Cargo caches that pull requests restore.
The job summary (`scripts/e2e/summary.py <output>` writes it to `summary.md`) lists each
journey and the bootstrap check with status and duration, then the totals. The
`backend-e2e-results-<attempt>` artifact keeps it with the JSON results.

`STEP_E2E_COVERAGE=1 scripts/e2e/run.sh`, as CI runs it, builds instrumented binaries into
`bin-coverage` and stops the services after the journeys so that they write LLVM profiles.
`coverage/summary.md` (also in the job summary) and `summary.json` give lines, functions and
regions per `packages/<package>/src` as `scripts/coverage` defines them, counting only source
linked into the binaries. The profile table shows each service's executed functions and exit
code (143: stopped by SIGTERM, counters kept by continuous mode); a missing profile fails the run.

The ordered tests in `scripts/e2e/journeys/test_journeys.py` cover tenant bootstrap,
event import and invalid bundles, voter import, automatic key generation,
private ballot publication, cast/revote/closing rules and results matching the
last accepted ballot. A combined-contest voting journey casts one native
ciphertext covering two contests, checks malformed ciphertext and foreign-domain
proof rejection before storage, then verifies voter weight batches through the
native trustees and independently calculated Velvet totals. Native archive
journeys cover configuration-only, publication-only and activity-log-only exports,
including older selective bundles with unused native keys. They reject missing
manager keys and tally exports without protocol boards. They also export a
completed election with its protocol boards, publications, voters and tally, then recover it through the
CLI and the authenticated encrypted-import API. They check preserved canonical
protocol messages, remapped board relationships, immutable signed configuration
domains and the same independently calculated vote totals. Archives also
retain publication metadata and ballot styles, rebuilding their immutable
storage objects under the recovered event's identifiers. Historical archives
without publication metadata recover their styles as generated, unpublished
publications. Encrypted archives contain manager identities; decrypted server
files belong to the import request
and are removed when it finishes, including on decryption failure.
The manual ceremony journey imports a completed event, rejects the original
public ceremony file, downloads a file scoped to the copy, and completes a new
tally over a fresh accepted vote. The server custody browser runner exercises
that recovery download and upload through the production administration portal.
Fixtures live in `fixtures.py`; clients use Python's
standard library. Add assertions against observable responses and persisted
results, with valid controls for rejection cases.
The same fixture and clients build the development stack's
[real-backend scenarios](../fast-feedback.md#real-backend-scenarios).

Every journey must pass. Errors, skipped journeys and expected failures all fail
the run; a green result means the complete selected sequence passed.

Concurrent launchers using the same explicit project on this host and user fail before touching Docker, even from different checkouts. A process-held project lock covers startup through cleanup and also protects `--down`; it releases automatically when the launcher exits. Runs on separate hosts or under different users must use distinct project names.
