---
sidebar_position: 6
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Real-stack browser journeys

Run `yarn --cwd packages install --frozen-lockfile`, then `scripts/e2e/ui/run.sh`. The runner builds the production voting, admin, results and verifier portals and starts the isolated backend services described in [Backend journeys](./backend-e2e.md). It drives Chromium against actual Keycloak, Hasura, Harvest, Windmill, trustees, PostgreSQL and MinIO; application responses come from those services. The admin fixture serves its optional react-admin telemetry image locally and checks the exact request, without contacting the vendor.

The journeys cover audit decoding, authenticated casting and stored receipts, private signed files, active-publication access rules, ballot lookup, verifier import, revote and closed-election rejection, administrator two-factor login using the isolated realm’s fixed test OTP, tally results publication and revocation. The tally oracle expects each voter's last accepted browser ballot. Unexpected HTTP and WebSocket destinations fail the run, including any telemetry request with a different method, path or query. One browser worker runs with no retries.

Use `--keep` to retain the synthetic stack after a failure and `--down` to remove it. `--skip-images`, `--skip-build` and `--skip-ui-build` reuse local artifacts while developing; rebuild after changing their source. `DOCKER='sudo docker'` supports hosts requiring elevated Docker access. Project names must remain isolated from development services; the default is `step-e2e-ui`.

Reports, failure screenshots and traces, and service logs are written under `.cache/backend-e2e/ui-run`. The workflow runs nightly, by manual dispatch, and for pull requests labeled `e2e-stack`; its cold backend build can take over an hour. All credentials and election data come from the disposable development fixture.
