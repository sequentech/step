---
id: ui-essentials
title: UI Essentials tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Test file paths in this guide are relative to [`packages/ui-essentials/tests/`](https://github.com/sequentech/step/blob/feat/meta-13302-ui-essentials-coverage/main/packages/ui-essentials/tests). Commands state their working directory.

Install the locked workspace dependencies from `packages/`, then run:

```bash
yarn --cwd ui-essentials test --runInBand
yarn --cwd ui-essentials test:coverage:baseline
yarn --cwd ui-essentials test:browser
TZ=America/Toronto yarn --cwd ui-essentials jest useSelectElectionCountdown.test.tsx --runInBand
```

The ordinary test command runs the full package/test type check before Jest.
It also runs in the existing frontend CI job. The browser runner needs Node
22.22 or newer and Google Chrome; `UI_ESSENTIALS_TEST_CHROME_PATH` can select a
different Chromium executable. The browser fixture serves synthetic data on
an ephemeral loopback port and blocks requests to other origins. No server
credentials or paid testing service is needed.

## Behavioral boundaries

The unit suite covers shared voting controls, modal confirmation, result
selection, preferential-round navigation, missing/invalid numeric results,
language changes, file imports and election countdowns. Existing static
accessibility tests remain part of the suite. New interaction tests use real
MUI components; selected tests stub translation copy or a UI Core utility
boundary explicitly.

The four Chromium tests use actual shared components and browser APIs. They
exercise keyboard activation of the native file picker, asynchronous JSON
failure and retry, drag-and-drop through a real `DataTransfer`, checkbox
callback counts and the countdown clock. This is component integration, not
an authenticated voter journey. Browser execution is separate from the Jest
coverage totals.

The countdown's fake-clock tests also run in Toronto's timezone to cover the
repeated hour when daylight saving time ends. Calendar month increments clamp
to the month's last day; the remaining weeks, days and clock fields describe
elapsed time. Already-expired dates return zero, invalid dates return null,
and timers stop at expiry or unmount. This display does not authorize voting;
the server remains responsible for election opening and closing.

File import callbacks may be synchronous or asynchronous. The component blocks
a second import while one is pending and displays a retry message on failure.
Callers can supply translated `errorMessage` copy. Raw parser errors are not
shown because they can contain private input data. The `accept` attribute is a
file-picker hint; callers must still validate file contents and enforce limits.

## Coverage gate

CI measures both actual PR revisions and rejects a decrease in lines, statements,
functions or branches independently. `test:coverage:baseline` writes reports
without enforcing the absolute local thresholds configured in Jest.

Coverage includes every runtime source module, including unimported modules
and translations. Only declarations, tests and Storybook examples are excluded.
Babel instruments executable source before transformation. HTML, JSON and LCOV
reports are written under `coverage/`.

For a fast development loop, run the affected Jest file first. Rerun Voting
Portal's browser tests when changing shared selection behavior. Preserve a
failing run against the previous implementation when fixing a regression.

## Resource and state transitions

Autocomplete controls distinguish selection from creation. Repeated input and
known labels must not request creation; choices loaded after mounting remain
available without losing local additions. Countdown tests observe layout commits
so that a passive-effect correction cannot hide a stale election deadline.

Dispatch a second file event before React commits its busy state to check
synchronous re-entry. The immediate guard owns the pending import and releases
it on both success and failure; UI state reflects that guard. Each wrapper uses
a native button, opens the picker once and displays translated import errors.
Parser details must not reach the rendered error message.

An initially selected label that is entered again remains available after its
chip is removed, even when the remote choices omit it. Repeated input neither
duplicates the option nor requests creation of an already selected label.
