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

Portal dev servers compile `@sequentech/ui-core` and `@sequentech/ui-essentials`
from `src`, so a shared component edit reaches every running portal without a
package build or server restart. `PORT` overrides a portal's default port and
`BROWSER=none` stops it opening a browser:

```sh
PORT=3100 BROWSER=none yarn --cwd packages/voting-portal start
```

React Refresh keeps component state when the edited module exports only
components; other edits reload the page. React, MUI, Emotion, router, i18n,
Apollo and `sequent-core` always resolve to the portal's own copy. Dev servers do
not type-check: run `test:types` in the voting portal, results portal or ballot
verifier (it resolves the shared sources), or build the admin portal.

Production builds and journeys still use the packages' `dist` entry points: run
`yarn --cwd packages build:ui-core` and `build:ui-essentials` before
`build:<portal>`. `STEP_SHARED_UI=dist` makes a dev server use those builds too,
for example to reproduce a production-only difference. The shared settings are in
`packages/ui-essentials/webpack.portal.cjs`.

## Screens, workbench and scenarios

## Incremental WASM

## Focused tests

## Benchmarks

## Incremental CI
