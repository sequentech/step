---
sidebar_position: 6
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Real-stack browser journeys

Run `yarn --cwd packages install --frozen-lockfile`, then `scripts/e2e/ui/run.sh`. The runner builds the production voting, admin, results and verifier portals and starts the isolated backend services described in [Backend journeys](./backend-e2e.md). It drives Chromium against actual Keycloak, Hasura, Harvest, Windmill, trustees, PostgreSQL and MinIO; application responses come from those services. The admin fixture serves its optional react-admin telemetry image locally and checks the exact request, without contacting the vendor.

The journeys cover audit decoding, authenticated casting and stored receipts, private signed files, active-publication access rules, ballot lookup, verifier import, revote and closed-election rejection, administrator two-factor login using the isolated realm’s fixed test OTP, tally results publication and revocation. The tally oracle expects each voter's last accepted browser ballot. Unexpected HTTP and WebSocket destinations fail the run, including any telemetry request with a different method, path or query. One browser worker runs with no retries.

Use `--keep` to retain the synthetic stack after a failure. To remove it, set `STEP_E2E_PROJECT` to the name printed at startup and run `scripts/e2e/ui/run.sh --down`; teardown requires an explicit project name. Remove a retained stack before reusing its explicit name. `--skip-images`, `--skip-build` and `--skip-ui-build` reuse local artifacts while developing; rebuild after changing their source. `DOCKER='sudo docker'` supports hosts requiring elevated Docker access. Project names must remain isolated from development services; the default is a unique `step-e2e-ui-...` name for each invocation. Existing projects are rejected without teardown, even if a prior run left logs or ownership markers in the selected output directory.

Reports, failure screenshots and traces, and service logs are written under `.cache/backend-e2e/<project>`. The workflow runs nightly, by manual dispatch, and for pull requests labeled `e2e-stack`; its cold backend build can take over an hour. All credentials and election data come from the disposable development fixture.
