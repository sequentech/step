---
name: step-e2e
description: Add or diagnose Step Playwright browser E2E tests, isolated devcontainer CI, or designated synthetic tenant probes. Use for product journeys, not capacity measurements or isolated unit tests.
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Read [the operating guide](../../../docs/docusaurus/docs/07-developers/e2e.md) and [scenario inventory](../../../packages/e2e/scenarios.yaml). Respect the requested target and branch. A planning-only request does not authorize execution.

Keep package scenarios in `packages/<portal>/test/e2e/*.spec.ts`, shared fixtures/config in `packages/e2e`, and journeys crossing portal boundaries in its `journeys/` folder. Reuse `test/load/flow.ts` for login, real candidate selection and encryption. Do not copy complete configurations into each portal.

Run `scripts/e2e images`, then `scripts/e2e run` in the E2E devcontainer/host Docker environment. Runs own uniquely named Compose projects and isolated CLI credentials. Inspect the recorded project before touching services; other running development stacks are not fixtures. `scripts/e2e cleanup RUN_ID` only removes the named run. Use `--keep` for local diagnosis, and clean it afterward. Do not reuse attempted voters or introduce browser retries after ambiguous casts.

Use role/name locators, actual readiness, and API/CLI setup for unrelated state. Keep the operation being tested in the UI. A cast requires real encryption, no demo acceptance, a GraphQL response without errors, matching UI/API identifiers and independent database persistence. Audit tests download a fresh ballot. Do not replace these checks with HTTP 200, screenshots or a success toast.

Migrate legacy assertions deliberately and update the scenario inventory. Keep an old driver while it still owns uncovered behavior. Loadero enrollment/report scenarios and the broader Nightwatch admin suites currently remain in that category.

For registered deployments use `scripts/e2e probe --target NAME`. It prepares and cleans only an owned election in the designated synthetic tenant; never run migrations, Compose teardown or database reset there. Targets are disabled until an operator supplies registry values, credentials, compatible image digest and monitoring routing. Current task scope leaves all real environments disabled.

Verify affected runner contracts, TypeScript, the narrow browser journey and its shared lifecycle. Record actual wall time, failing stage and missing evidence. Keep credentials, raw traces and service logs under the run's private directory; publish only approved reports. Do not claim application success from contract tests or test discovery.
