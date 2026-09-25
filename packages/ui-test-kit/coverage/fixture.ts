// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test as base, type Page} from "@playwright/test"
import {mergeProcessCovs, type ProcessCov, type ScriptCov} from "@bcoe/v8-coverage"
import {existsSync} from "node:fs"
import {mkdir, writeFile} from "node:fs/promises"
import {join, resolve, sep} from "node:path"
import {pathToFileURL} from "node:url"
import {servedRoots} from "../server/static"

type ScriptCoverage = Awaited<ReturnType<Page["coverage"]["stopJSCoverage"]>>[number]
interface CoverageRecorder {
    add(entries: ScriptCoverage[]): void
}
/** A worker's merged V8 result, plus how many tests contributed to it. */
export interface JourneyCoverage extends ProcessCov {
    tests: number
}

/** `STEP_UI_JOURNEY_COVERAGE=1` records V8 coverage of the served production bundles. */
export const journeyCoverage = process.env.STEP_UI_JOURNEY_COVERAGE === "1"

// Only scripts served from a dist directory are kept, keyed by their file so
// the report can read the adjacent source map. Inline and blob scripts drop out.
function bundleScripts(entries: ScriptCoverage[]): ScriptCov[] {
    const scripts: ScriptCov[] = []
    for (const {url, scriptId, functions} of entries) {
        const parsed = URL.canParse(url) ? new URL(url) : undefined
        const root = parsed && servedRoots.get(parsed.origin)
        if (!parsed || !root) continue
        const file = resolve(root, `.${decodeURIComponent(parsed.pathname)}`)
        if (file.startsWith(root + sep) && file.endsWith(".js") && existsSync(file))
            scripts.push({scriptId, url: pathToFileURL(file).href, functions})
    }
    return scripts
}

export const test = base.extend<{journeyCoverage: void}, {coverageRecorder: CoverageRecorder}>({
    coverageRecorder: [
        // Playwright requires an object pattern even for a fixture without dependencies.
        // eslint-disable-next-line no-empty-pattern
        async ({}, use, workerInfo) => {
            // Merging after every test keeps one copy of each bundle's counters per worker.
            let merged: JourneyCoverage = {tests: 0, result: []}
            await use({
                add: (entries) => {
                    const {result} = mergeProcessCovs([merged, {result: bundleScripts(entries)}])
                    merged = {tests: merged.tests + 1, result}
                },
            })
            if (!merged.tests) return
            const directory = join(workerInfo.project.outputDir, "journey-coverage")
            await mkdir(directory, {recursive: true})
            await writeFile(
                join(directory, `v8-worker-${workerInfo.workerIndex}.json`),
                JSON.stringify(merged)
            )
        },
        {scope: "worker"},
    ],
    journeyCoverage: [
        async ({page, browserName, coverageRecorder}, use) => {
            if (!journeyCoverage || browserName !== "chromium") return use()
            await page.coverage.startJSCoverage({resetOnNavigation: false})
            await use()
            if (!page.isClosed()) coverageRecorder.add(await page.coverage.stopJSCoverage())
        },
        {auto: true},
    ],
})
