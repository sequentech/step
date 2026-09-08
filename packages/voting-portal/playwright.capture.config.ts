// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {resolve} from "node:path"
import {defineConfig} from "@playwright/test"

// A diagnostic capture always records successful requests and never retries a cast.
export default defineConfig({
    testDir: "./test/load",
    testMatch: ["capture.spec.ts", "status.spec.ts"],
    workers: 1,
    retries: 0,
    timeout: 180_000,
    expect: {timeout: 15_000},
    outputDir: resolve(process.env.CAPTURE_OUTPUT_DIR || ".cache/capture", "playwright"),
    reporter: "line",
})
