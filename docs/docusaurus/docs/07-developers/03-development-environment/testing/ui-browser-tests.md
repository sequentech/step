---
id: ui-browser-tests
title: UI browser tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Storybook runs the UI Essentials and portal stories in Chromium through Vitest.
Use Node 22.22 and Yarn 1.22.22, then from the repository root:

```sh
yarn --cwd packages install --frozen-lockfile
yarn --cwd packages/ui-essentials playwright install chromium
yarn --cwd packages/ui-essentials test:stories --coverage
yarn --cwd packages/ui-essentials build-storybook
```

Replace `ui-essentials` with `voting-portal`, `admin-portal`, `results-portal` or
`ballot-verifier`. `yarn --cwd packages/<package> storybook` opens the interactive
catalog. CI runs interactions, accessibility checks and a catalog build for each
package using the pinned Playwright image. Its JUnit, screenshots and separate
Istanbul coverage reports are uploaded from `test-results/`.

Place typed `*.stories.tsx` beside components, or under their `__stories__`
directory. Use `storybook/test` assertions and spies, query accessible names, and
assert rendered outcomes and callback values. The shared preview supplies the
theme, deterministic English translations and an in-memory router. Configure
`parameters.router` for route parameters and initial history. Browser contexts
use an English locale and UTC. Mock external services; initialize real WASM in a
story loader when the component needs it. Tests block unexpected network requests;
only local module, image, font and WASM assets may reach the server.

Stories are excluded from production type builds and the existing Jest coverage
profile. Storybook coverage is reported separately from that gate. Shared test
setup lives in `packages/test-support/storybook/`. Keep changes to production behavior separate from story maintenance.
Known accessibility defects use `parameters.expectedFailure` with a reason and
the exact axe rule IDs. The Vitest hook marks only matching failures as expected;
unrelated failures still fail, and a repaired defect causes an unexpected-pass
failure until its marker is removed. Use `expectedFailure: null` to clear a marker inherited
from the component meta.
