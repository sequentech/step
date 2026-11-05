// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {defineConfig} from "@playwright/test"

export default defineConfig({
    testDir: "./tests",
    fullyParallel: true,
    workers: 2,
    reporter: [["list"], ["junit", {outputFile: "test-results/contracts.xml"}]],
})
