// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Markdown summaries of browser test results and coverage, appended to the
// GitHub step summary when it exists and always printed:
//   node --experimental-strip-types summary.mts stories <package-dir>
//   node --experimental-strip-types summary.mts journeys <package-dir>
//   node --experimental-strip-types summary.mts results <junit.xml> <title>

import {existsSync, readFileSync} from "node:fs"
import {appendFile, readFile, readdir, realpath, writeFile} from "node:fs/promises"
import {basename, join} from "node:path"
import type {ProcessCov} from "@bcoe/v8-coverage"
import type {CoverageSummaryData} from "istanbul-lib-coverage"
import {journeyCoverage} from "./istanbul.mts"

type Status = "passed" | "expected failure" | "failed" | "skipped"
interface TestCase {
    name: string
    file: string
    seconds: number
    status: Status
}
const statuses: Status[] = ["passed", "expected failure", "failed", "skipped"]
const plural: Record<Status, string> = {
    "passed": "passed",
    "expected failure": "expected failures",
    "failed": "failed",
    "skipped": "skipped",
}
const metrics = ["lines", "statements", "functions", "branches"] as const

const entities: Record<string, string> = {amp: "&", lt: "<", gt: ">", quot: '"', apos: "'"}
const decode = (text: string) =>
    text.replace(/&(#x[\da-f]+|#\d+|\w+);/gi, (entity, name: string) =>
        name.startsWith("#")
            ? String.fromCodePoint(Number(name.replace(/^#x/i, "0x").replace(/^#/, "")))
            : (entities[name] ?? entity)
    )

// Playwright's test.fail() and the story expectedFailure hook both leave a
// passing case with an annotation property; an unexpected pass is a failure.
async function readJUnit(file: string) {
    const xml = (await readFile(file, "utf8")).replace(/<!\[CDATA\[[\s\S]*?\]\]>/g, "")
    const cases: TestCase[] = []
    for (const [, attributes, body = ""] of xml.matchAll(
        /<testcase\b([^>]*?)(?:\/>|>([\s\S]*?)<\/testcase>)/g
    )) {
        const attribute = (name: string) =>
            decode(new RegExp(`\\b${name}="([^"]*)"`).exec(attributes)?.[1] ?? "")
        let status: Status = "passed"
        if (/<skipped\b/.test(body)) status = "skipped"
        else if (/<(?:failure|error)\b/.test(body)) status = "failed"
        else if (/<property\s+name="(?:fail|expected-failure)"/.test(body))
            status = "expected failure"
        const seconds = Number(attribute("time")) || 0
        cases.push({name: attribute("name"), file: attribute("classname"), seconds, status})
    }
    const total = /<testsuites\b[^>]*?\btime="([^"]*)"/.exec(xml)?.[1]
    const seconds = total ? Number(total) : cases.reduce((sum, test) => sum + test.seconds, 0)
    return {cases, seconds}
}

const count = (cases: TestCase[], status: Status) =>
    cases.filter((test) => test.status === status).length
const duration = (seconds: number) => `${seconds.toFixed(1)} s`
const table = (header: string[], rows: (string | number)[][], textColumns = 1) =>
    [header, header.map((_, index) => (index < textColumns ? "---" : "---:")), ...rows]
        .map((row) => `| ${row.join(" | ")} |`)
        .join("\n")

function resultsTable(noun: string, {cases, seconds}: Awaited<ReturnType<typeof readJUnit>>) {
    const header = [noun, "Passed", "Expected failures", "Failed", "Skipped", "Time"]
    const row = [cases.length, ...statuses.map((status) => count(cases, status)), duration(seconds)]
    return table(header, [row])
}

const ratio = ({covered, total, pct}: CoverageSummaryData[(typeof metrics)[number]]) =>
    `${covered}/${total} (${total ? `${pct}%` : "n/a"})`

async function publish(markdown: string) {
    if (process.env.GITHUB_STEP_SUMMARY)
        await appendFile(process.env.GITHUB_STEP_SUMMARY, `${markdown}\n\n`)
    process.stdout.write(`${markdown}\n\n`)
}

async function stories(packageDir: string) {
    const results = join(packageDir, "test-results")
    const junit = await readJUnit(join(results, "stories.xml"))
    const coverage = JSON.parse(
        await readFile(join(results, "coverage", "coverage-summary.json"), "utf8")
    ) as {total: CoverageSummaryData}
    const istanbul = metrics.map((metric) => {
        const {covered, total, pct} = coverage.total[metric]
        return [metric[0].toUpperCase() + metric.slice(1), covered, total, total ? pct : "n/a"]
    })
    await publish(
        [
            `### ${basename(packageDir)} stories`,
            resultsTable("Stories", junit),
            "Istanbul coverage of `src/**` (stories and tests excluded):",
            table(["Metric", "Covered", "Total", "%"], istanbul),
        ].join("\n\n")
    )
}

async function results(file: string, title: string) {
    const junit = await readJUnit(file)
    const rows = junit.cases.map((test) => [
        `${test.file} › ${test.name}`.replace(/\|/g, "\\|"),
        test.status,
        duration(test.seconds),
    ])
    const totals = statuses.map((status) => `${count(junit.cases, status)} ${plural[status]}`)
    rows.push([`**${junit.cases.length} journeys**`, totals.join(", "), duration(junit.seconds)])
    await publish([`### ${title}`, table(["Journey", "Status", "Duration"], rows, 2)].join("\n\n"))
}

function readJson<T>(file: string): T {
    return JSON.parse(readFileSync(file, "utf8")) as T
}

async function journeys(target: string) {
    // The fixture records real paths, as the dist server resolves them.
    const packageDir = await realpath(target)
    const name = basename(packageDir)
    const results = join(packageDir, "test-results")
    const junit = await readJUnit(join(results, "journeys.xml"))
    const raw = join(results, "journey-coverage")
    const workers = existsSync(raw)
        ? (await readdir(raw))
              .filter((file) => /^v8-.*\.json$/.test(file))
              .map((file) => readJson<ProcessCov & {tests: number}>(join(raw, file)))
        : []
    const measured = workers.reduce((sum, worker) => sum + worker.tests, 0)
    const sections = [`### ${name} production journeys`, resultsTable("Journeys", junit)]
    if (!measured) {
        sections.push("No journey coverage was recorded; set `STEP_UI_JOURNEY_COVERAGE=1`.")
        await publish(sections.join("\n\n"))
        return
    }
    const {coverage, unloaded, unverified} = await journeyCoverage(packageDir, workers)
    const total = coverage.getCoverageSummary().toJSON()
    const files = Object.fromEntries(
        coverage.files().map((path) => [path, coverage.fileCoverageFor(path).toSummary().toJSON()])
    )
    await writeFile(join(raw, "coverage-summary.json"), JSON.stringify({total, ...files}, null, 2))
    const notes = [
        `V8 coverage from ${measured} tests of the production bundle. Statements, functions`,
        "and branches are Istanbul's, from the portal's TypeScript; each counts the",
        "innermost V8 block around its bundle code, traced through the source maps.",
        unloaded ? `Code in ${unloaded} lazy chunk(s) that never loaded counts as uncovered.` : "",
        unverified ? `${unverified} module(s) could not be re-emitted and keep ES5 positions.` : "",
    ]
    sections.push(
        notes.filter(Boolean).join(" "),
        table(
            ["Source", "Lines", "Statements", "Functions", "Branches"],
            [[`${name}/src`, ...metrics.map((metric) => ratio(total[metric]))]]
        )
    )
    await publish(sections.join("\n\n"))
}

const [mode, target, title] = process.argv.slice(2)
const reports: Record<string, [heading: string, write: () => Promise<void>]> = {
    stories: [`${basename(target ?? "")} stories`, () => stories(target)],
    journeys: [`${basename(target ?? "")} production journeys`, () => journeys(target)],
    results: [title ?? basename(target ?? ""), () => results(target, title ?? basename(target))],
}
const report = target ? reports[mode] : undefined
if (!report) {
    process.stderr.write(
        "Usage: summary.mts stories|journeys <package-dir> | results <junit.xml> [title]\n"
    )
    process.exitCode = 2
} else {
    try {
        await report[1]()
    } catch (error) {
        // A missing report means an earlier step failed; that step gates the job.
        const {code, path} = error as NodeJS.ErrnoException
        if (code !== "ENOENT") throw error
        await publish(`### ${report[0]}\n\nNo report at \`${path}\`.`)
    }
}
