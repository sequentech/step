// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {defineConfig, devices} from "@playwright/test"

export default defineConfig({
    testDir: "./tests/journeys",
    fullyParallel: false,
    workers: 1,
    timeout: 45000,
    expect: {timeout: 10000},
    retries: 0,
    reporter: [["list"], ["junit", {outputFile: "test-results/journeys.xml"}]],
    use: {
        ...devices["Desktop Chrome"],
        locale: "en-US",
        timezoneId: "UTC",
        serviceWorkers: "block",
        trace: "retain-on-failure",
        screenshot: "only-on-failure",
    },
})
