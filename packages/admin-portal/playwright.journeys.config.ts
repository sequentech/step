// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {defineConfig, devices} from "@playwright/test"

// A local Chromium (e.g. from nix) where Playwright can't download its own; CI leaves it unset.
const executablePath = process.env.CHROMIUM_EXECUTABLE_PATH

export default defineConfig({
    testDir: "./test/journeys",
    // Keep local memory bounded; CI runners have four CPUs, and CI also splits
    // this suite into shards.
    fullyParallel: true,
    workers: process.env.CI ? 3 : 2,
    // The CI shards run with V8 coverage on two-core runners, where a journey takes
    // about 13s and the slower ones past 40s; the budgets leave room for that.
    timeout: 90000,
    expect: {timeout: 30000},
    retries: 0,
    reporter: [["list"], ["junit", {outputFile: "test-results/journeys.xml"}]],
    use: {
        ...devices["Desktop Chrome"],
        locale: "en-US",
        timezoneId: "UTC",
        serviceWorkers: "block",
        trace: "retain-on-failure",
        screenshot: "only-on-failure",
        ...(executablePath ? {launchOptions: {executablePath}} : {}),
    },
})
