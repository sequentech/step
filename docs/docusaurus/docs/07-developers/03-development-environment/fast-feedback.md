---
id: fast-feedback
title: Fast feedback loops
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The development loop is edit, see the result, run the relevant test. Commands in
this guide run from the repository root inside the devcontainer unless stated;
`scripts/dev/step-dev <command> --help` describes each one.

## Devcontainer modes

## Shared UI hot reload

## Screens, workbench and scenarios

Production voter screens open against synthetic elections without login or services.
Scenarios in `packages/ui-test-kit/fixtures/scenarios.ts` build versioned snapshots:
`version`, `scenarioId`, `provenance`, `tenantId`, `areaId`, `channel` and a
publication `preview` document. `VoterPreview` in `packages/voting-portal/src/preview/`
serves Storybook and the workbench: portal theme, authentication disabled, a voter on
the snapshot's channel, a GraphQL client that rejects every operation, the production
WASM gate and store, and the portal's publication preview loader.

```sh
yarn --cwd packages/workbench dev            # 127.0.0.1:5173, or WORKBENCH_PORT
yarn --cwd packages/voting-portal storybook  # localhost:6007
yarn --cwd packages/workbench test           # policy, snapshot and storage units
yarn --cwd packages/workbench test:smoke     # Playwright flow, reuses a running dev server
```

The workbench link `#/scenario/<scenario>/<screen>` and the story
`scenarios-<scenario>--<screen>` (title `Scenarios/<scenario title>`) open the same
screen. Scenarios are `simple-plurality`, `ranked-multi-contest` and `kiosk-voter`;
screens are `chooser`, `start`, `vote`, `review` and `confirmation`. Review and
confirmation open with the first valid choices encrypted. The `Voter channel` toolbar
switches any story between online and kiosk voters.

The workbench imports, exports and resets snapshots, overrides contest policies and
vote bounds, shows the store and the voting screen's validation, and runs interpret,
Next checks, encrypt, hash and decode on the current selection with sequent-core.
Exports carry the overrides in the document and list them in `provenance.changes`.
Its local storage keys start with `sequent.workbench.v1.`; Reset removes them and the
portal's session storage. Requests to other origins and non-GET requests are refused.
Workbench controls have stories under `Workbench/`.

`WORKBENCH_SEQUENT_CORE=<wasm-pack web output>` loads another sequent-core build
without reinstalling; the page reloads when its files change and the inspector shows
the binary's hash. `WORKBENCH_TEST_CHROME_PATH` selects a local Chromium for
`test:smoke`. Stories render one production route with its action; the workbench mounts
the production event routes. The only preview UI inside the portal frame is the error
shown when the portal loader rejects a snapshot.

## Incremental WASM

## Focused tests

## Benchmarks

## Incremental CI
