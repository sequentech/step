// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {defineConfig, devices} from "@playwright/test"

export default defineConfig({
    testDir: "./tests",
    fullyParallel: false,
    workers: 1,
    retries: 0,
    timeout: 120000,
    expect: {timeout: 15000},
    reporter: [["list"]],
    use: {
        ...devices["Desktop Chrome"],
        launchOptions: {executablePath: process.env.CHROMIUM_EXECUTABLE_PATH},
        locale: "en-US",
        timezoneId: "UTC",
        screenshot: "only-on-failure",
    },
})
