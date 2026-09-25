// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {defineConfig, devices} from "@playwright/test"
import {resolve} from "node:path"

const output = process.env.STEP_UI_E2E_OUTPUT_DIR ?? "/out"
export default defineConfig({
    testDir: "./tests",
    fullyParallel: false,
    workers: 1,
    retries: 0,
    timeout: 120000,
    expect: {timeout: 15000},
    outputDir: resolve(output, "test-results"),
    reporter: [
        ["list"],
        ["json", {outputFile: resolve(output, "ui-journeys.json")}],
        ["junit", {outputFile: resolve(output, "ui-journeys.xml")}],
    ],
    use: {
        ...devices["Desktop Chrome"],
        actionTimeout: 15000,
        navigationTimeout: 30000,
        locale: "en-US",
        timezoneId: "UTC",
        serviceWorkers: "block",
        trace: "retain-on-failure",
        screenshot: "only-on-failure",
    },
})
