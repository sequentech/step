// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {defineConfig} from "@playwright/test"

/** Concurrency and finite voter ownership are managed inside the shard worker. */
export default defineConfig({
    testDir: "./test/load",
    testMatch: "scale.spec.ts",
    workers: 1,
    retries: 0,
    expect: {timeout: Number(process.env.LOAD_ACTION_TIMEOUT_MS ?? 15_000)},
    reporter: "line",
    outputDir: process.env.LOAD_ARTIFACTS,
})
