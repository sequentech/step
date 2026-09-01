// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {defineConfig} from "@playwright/test"

// Dedicated config for the online-channel load test (test/load), normally
// driven by packages/step-cli/scripts/run-online-load-test.sh — kept separate
// so `yarn test` (Jest) and the Nightwatch e2e suites are unaffected.
export default defineConfig({
    testDir: "./test/load",
    fullyParallel: true,
    // A retried voter would attempt a second vote and be rejected as a
    // duplicate, skewing the results — report the failure instead.
    retries: 0,
    timeout: Number(process.env.LOAD_TEST_VOTE_TIMEOUT_MS ?? 180_000),
    expect: {timeout: 15_000},
    outputDir: process.env.LOAD_TEST_OUT_DIR
        ? `${process.env.LOAD_TEST_OUT_DIR}/traces`
        : "./test-results/load",
    use: {
        headless: process.env.LOAD_TEST_HEADED !== "true",
        // Failure diagnostics only: recording every successful voter would
        // add disk and CPU cost proportional to the load being generated.
        trace: "retain-on-failure",
        screenshot: "only-on-failure",
        video: "off",
        actionTimeout: 30_000,
        navigationTimeout: 60_000,
    },
})
