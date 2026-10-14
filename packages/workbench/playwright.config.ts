// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {defineConfig, devices} from "@playwright/test"

const port = Number(process.env.WORKBENCH_PORT ?? 5173)
const baseURL = `http://127.0.0.1:${port}`

export default defineConfig({
    testDir: "./tests",
    fullyParallel: false,
    workers: 1,
    timeout: 90000,
    expect: {timeout: 15000},
    retries: 0,
    reporter: [["list"], ["junit", {outputFile: "test-results/smoke.xml"}]],
    use: {
        ...devices["Desktop Chrome"],
        baseURL,
        viewport: {width: 1440, height: 900},
        locale: "en-US",
        timezoneId: "UTC",
        serviceWorkers: "block",
        trace: "retain-on-failure",
        screenshot: "only-on-failure",
        launchOptions: {executablePath: process.env.WORKBENCH_TEST_CHROME_PATH || undefined},
    },
    // A workbench already serving this port is reused, e.g. the one a developer is editing.
    webServer: {
        command: "vite",
        env: {WORKBENCH_PORT: String(port)},
        url: baseURL,
        reuseExistingServer: true,
        timeout: 120000,
    },
})
