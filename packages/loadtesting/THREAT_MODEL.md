<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# loadtesting threat model

loadtesting defines the load-test container image (`Dockerfile`, `entrypoint.sh`). Its own code is a Nightwatch test (`nightwatch/src/voting.js`) that drives headless Chromium through the Keycloak login and the voting portal and casts ballots as synthetic voters, with parallel workers started by `run_bg_voting.sh`. The image also bundles `step-cli`, the `e2e` binary and `load-tool`, which act with whatever credentials they are given. Operators run it on load-test hosts and in the devcontainer `loadtest` service, the manually dispatched `E2E Tests` workflow runs it in CI, and the shared build workflows build and publish it. It holds no privilege of its own, but every ballot it casts is a real ballot in the target event, and the credentials it is given decide what else it can change. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Target election event and deployment**: ballots cast and load generated against `VOTING_URL`. Integrity of the ballot box; availability for real voters.
- **Synthetic voter credentials and run artefacts**: `USERNAME_PATTERN` and `PASSWORD_PATTERN` expanded per voter index; run logs, `used_voters.txt`, optional screenshots. Confidentiality wherever those accounts exist.
- **Operator credentials given to the container**: database credentials in the environment for `load-tool` and the `step-cli` direct-database commands; the `step-cli` session stored next to the binary in `/opt/sequentech/config/`. Confidentiality: they give direct database or admin access.
- **The image**: binaries, browser, npm and Python packages, and what the build downloads. Integrity, since it runs with the credentials above.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `entrypoint.sh` subcommands `vote-cast`, `e2e` (both start `run_bg_voting.sh`), `load-tool`, `step-cli`, `shell`, `sleep` | Whoever starts the container: operator, `E2E Tests` workflow, devcontainer | operator | Subcommand allowlist in `case "$SUBCOMMAND"`; arguments forwarded as `"$@"`, never evaluated; `shell` deliberately gives a full shell |
| `run_bg_voting.sh` flags (`--voting-url`, patterns, `--base-test`, `--previous-voters-file`); `nightwatch/src/voting.js` `Automated Voting Test` | Operator; Nightwatch workers | operator | `is_positive_int` on `--batches` and `--instances`; unknown flags rejected; values passed to workers as environment |
| Pages served at `VOTING_URL` (Keycloak login, voting portal) | Target deployment | untrusted | Rendered in headless Chromium in the container; no certificate-relaxing flags in `nightwatch/nightwatch.conf.js` |
| Bundled `load-tool` and `step-cli` | Operator with store or admin credentials | operator | The image sets no credentials (`Dockerfile` only sets `CHROME` and `PATH`) |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| loadtesting-T1 | Tampering | Ballots cast by a load run stay in the target event, and synthetic voter accounts left in an event that goes live can be misused | The test is an ordinary voter client, so Keycloak login and server-side casting checks apply to it as to the voting portal | Accepted (by design: a load generator must reach real deployments; choosing a test event and removing synthetic voters and ballots are operator duties) |
| loadtesting-T2 | Denial of service | A run against a shared or production deployment during voting degrades service for real voters | `run_bg_voting.sh` requires positive integers for `--instances` and `--batches`, with no upper bound | Accepted (by design: generating load is the purpose; scheduling and target are operator decisions) |
| loadtesting-T3 | Information disclosure | Synthetic voter credentials leak through command lines, environment variables, workflow inputs or logs | Screenshots only with `--save-screenshots` | Partial |
| loadtesting-T4 | Tampering | A compromised package or script fetched during the image build ends up in an image that runs with operator credentials | Multi-stage `Dockerfile` limits what reaches the runtime image | Partial |
| loadtesting-T5 | Elevation of privilege | Bundled administration tools, given production credentials, change election data | No credentials in the image; `step-cli` reads database credentials from the environment at run time (`../step-cli/src/utils/hasura.rs`, `../step-cli/src/utils/keycloak.rs`) | Partial |
| loadtesting-T6 | Elevation of privilege | A malicious or compromised target page exploits the browser and takes over the container and the credentials in it | Headless Chromium in a container, run as root with `--no-sandbox` (`nightwatch/nightwatch.conf.js`) | Accepted (by design: the browser only loads the deployment under test, which the operator chooses) |

## Assumptions

- Operators point runs only at test events with synthetic voters and remove those voters and their ballots, or the event, before anything goes live (admin-portal, step-cli).
- Keycloak, harvest and Hasura enforce authentication, eligibility and the revote policy for these sessions as for the voting portal.
- Production store credentials and admin sessions are never given to this image, and it runs on hosts and networks separate from production.
- Only repository writers can dispatch the `E2E Tests` workflow; the container runtime isolates the container from its host, and the devcontainer `loadtest` service is used only in development.

## Review focus

- Image build steps and the provenance of what they install.
- Bundled administration tools and the credentials given to the container in each place it runs.
- Credential handling in arguments, environment and logs, locally and in CI.
- Cleanup of synthetic voters and ballots after a run.
