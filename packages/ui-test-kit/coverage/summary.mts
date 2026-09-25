// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Markdown summaries of browser test results and coverage, appended to the
// GitHub step summary when it exists and always printed:
//   node --experimental-strip-types summary.mts stories <package-dir>
//   node --experimental-strip-types summary.mts journeys <package-dir>
//   node --experimental-strip-types summary.mts results <junit.xml> <title>

import {existsSync, readFileSync} from "node:fs"
import {appendFile, readFile, readdir, realpath, writeFile} from "node:fs/promises"
import {basename, join, resolve, sep} from "node:path"
import {fileURLToPath} from "node:url"
import {mergeProcessCovs, type ProcessCov} from "@bcoe/v8-coverage"
import remapping, {type DecodedSourceMap, type EncodedSourceMap} from "@jridgewell/remapping"
import libCoverage, {type CoverageSummaryData} from "istanbul-lib-coverage"
import type {FileCoverageData, Range} from "istanbul-lib-coverage"
import ts from "typescript"
import v8ToIstanbul from "v8-to-istanbul"

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

// Webpack names sources `webpack://<package name>/<path from the package>`,
// with a query suffix on some assets.
function absoluteSources(map: EncodedSourceMap, packageDir: string): EncodedSourceMap {
    const prefix = `webpack://${readJson<{name: string}>(join(packageDir, "package.json")).name}/`
    const sources = map.sources.map((source) =>
        source?.startsWith(prefix)
            ? resolve(packageDir, source.slice(prefix.length).replace(/\?[^/]*$/, ""))
            : source
    )
    return {...map, sourceRoot: undefined, sources}
}

// ts-loader emits modules without source maps, so the bundle maps end at its
// ES5 output. Emitting the checked-out source again must reproduce that output
// exactly; the new map then continues the trace to the TypeScript.
function typescriptMaps(packageDir: string) {
    const config = ts.readConfigFile(join(packageDir, "tsconfig.json"), ts.sys.readFile)
    const {options: project, fileNames} = ts.parseJsonConfigFileContent(
        config.config,
        ts.sys,
        packageDir
    )
    const options: ts.CompilerOptions = {
        ...project,
        noEmit: false,
        sourceMap: true,
        declaration: false,
        declarationMap: false,
        emitDeclarationOnly: false,
        skipLibCheck: true,
        suppressOutputPathCheck: true,
    }
    const code = (output?: string) => output?.replace(/\/\/# sourceMappingURL=\S+\s*$/, "")
    let program: ts.Program | undefined
    return (path: string, emitted: string): EncodedSourceMap | undefined => {
        const text = readFileSync(path, "utf8")
        let {outputText: js, sourceMapText: map} = ts.transpileModule(text, {
            compilerOptions: options,
            fileName: path,
        })
        // Without transpileOnly, ts-loader emits through a program, which also
        // drops imports that are only used as types.
        if (code(js) !== emitted) {
            program ??= ts.createProgram({rootNames: fileNames, options})
            const source = program.getSourceFile(path)
            if (source)
                program.emit(source, (file, output) => {
                    if (file.endsWith(".map")) map = output
                    else js = output
                })
        }
        if (code(js) !== emitted || !map) return
        return {...JSON.parse(map), sources: [path], sourcesContent: [text], sourceRoot: undefined}
    }
}

const statementKinds = new Set([
    ts.SyntaxKind.BreakStatement,
    ts.SyntaxKind.ContinueStatement,
    ts.SyntaxKind.DebuggerStatement,
    ts.SyntaxKind.DoStatement,
    ts.SyntaxKind.EnumDeclaration,
    ts.SyntaxKind.EnumMember,
    ts.SyntaxKind.ExpressionStatement,
    ts.SyntaxKind.ForInStatement,
    ts.SyntaxKind.ForOfStatement,
    ts.SyntaxKind.ForStatement,
    ts.SyntaxKind.IfStatement,
    ts.SyntaxKind.LabeledStatement,
    ts.SyntaxKind.ReturnStatement,
    ts.SyntaxKind.SwitchStatement,
    ts.SyntaxKind.ThrowStatement,
    ts.SyntaxKind.TryStatement,
    ts.SyntaxKind.WhileStatement,
    ts.SyntaxKind.WithStatement,
])

// The statements Istanbul's instrumenter counts: executable statements,
// initializers, arrow expression bodies and the assignments emitted for enums.
// Unlike v8-to-istanbul's source lines, a translation table is one statement.
function statements(path: string, text: string) {
    const file = ts.createSourceFile(path, text, ts.ScriptTarget.Latest, true)
    const ranges: Range[] = []
    const position = (offset: number) => {
        const {line, character} = file.getLineAndCharacterOfPosition(offset)
        return {line: line + 1, column: character}
    }
    const visit = (node: ts.Node) => {
        let statement: ts.Node | undefined
        if (statementKinds.has(node.kind)) statement = node
        else if (ts.isVariableDeclaration(node) || ts.isPropertyDeclaration(node))
            statement = node.initializer
        else if (ts.isArrowFunction(node) && !ts.isBlock(node.body)) statement = node.body
        if (statement)
            ranges.push({start: position(statement.getStart(file)), end: position(statement.end)})
        ts.forEachChild(node, visit)
    }
    visit(file)
    return ranges
}

function bundleMap(
    file: string,
    packageDir: string,
    counted: (path: string) => boolean,
    typescript: ReturnType<typeof typescriptMaps>
) {
    const root = absoluteSources(readJson<EncodedSourceMap>(`${file}.map`), packageDir)
    const modules = new Set(
        root.sources.filter(
            (source): source is string => !!source && counted(source) && /\.tsx?$/.test(source)
        )
    )
    const emitted = new Map(
        root.sources.map((source, index) => [source, root.sourcesContent?.[index]])
    )
    const traced = new Set<string>()
    let unverified = 0
    const composed = remapping(
        root,
        (source) => {
            const content = emitted.get(source)
            if (traced.has(source) || !modules.has(source) || !content) return
            traced.add(source)
            const map = typescript(source, content)
            if (!map) unverified++
            return map
        },
        {decodedMappings: true}
    )
    // v8-to-istanbul attributes ranges whose ends map to different sources to
    // source 0, so an empty placeholder takes that slot.
    const sources = ["", ...composed.sources]
    const contents = ["", ...(composed.sourcesContent ?? [])]
    const map: DecodedSourceMap = {
        version: 3,
        file: composed.file,
        names: composed.names,
        sources,
        sourcesContent: sources.map((source, index) =>
            source && counted(source) ? (contents[index] ?? null) : null
        ),
        mappings: (composed.mappings as DecodedSourceMap["mappings"]).map((line) =>
            line.map((segment) =>
                segment.length === 1
                    ? segment
                    : segment.length === 4
                      ? [segment[0], segment[1] + 1, segment[2], segment[3]]
                      : [segment[0], segment[1] + 1, segment[2], segment[3], segment[4]]
            )
        ),
    }
    // Source lines that emit bundle code; tree-shaken code never runs.
    const code = new Map<string, Set<number>>()
    const countedSources = map.sources.map((source) => (source && counted(source) ? source : ""))
    for (const line of map.mappings)
        for (const segment of line) {
            if (segment.length === 1) continue
            const source = countedSources[segment[1]]
            if (source) code.set(source, (code.get(source) ?? new Set()).add(segment[2] + 1))
        }
    const text = new Map(sources.map((source, index) => [source, contents[index] ?? ""]))
    return {map, code, text, unverified}
}

// v8-to-istanbul reports every source line as a statement; keep its hit counts
// for the Istanbul statements that emit bundle code.
function bundleStatements(
    data: FileCoverageData,
    text: string,
    code: Set<number>,
    loaded: boolean
) {
    const hits = new Map<number, number>()
    for (const [key, range] of Object.entries(data.statementMap))
        hits.set(range.start.line, loaded ? data.s[key] : 0)
    const statementMap: FileCoverageData["statementMap"] = {}
    const s: FileCoverageData["s"] = {}
    let index = 0
    for (const range of statements(data.path, text)) {
        if (!code.has(range.start.line)) continue
        statementMap[index] = range
        s[index++] = hits.get(range.start.line) ?? 0
    }
    return {...data, statementMap, s}
}

async function journeys(target: string) {
    // The fixture records real paths, as the dist server resolves them.
    const packageDir = await realpath(target)
    const results = join(packageDir, "test-results")
    const junit = await readJUnit(join(results, "journeys.xml"))
    const raw = join(results, "journey-coverage")
    const workers = existsSync(raw)
        ? await Promise.all(
              (await readdir(raw))
                  .filter((file) => /^v8-.*\.json$/.test(file))
                  .map((file) => readJson<ProcessCov & {tests: number}>(join(raw, file)))
          )
        : []
    const measured = workers.reduce((sum, worker) => sum + worker.tests, 0)
    const executed = new Map(
        mergeProcessCovs(workers).result.map((script) => [
            fileURLToPath(script.url),
            script.functions,
        ])
    )
    const name = basename(packageDir)
    const src = join(packageDir, "src") + sep
    const counted = (path: string) => path.startsWith(src)
    const coverage = libCoverage.createCoverageMap({})
    const typescript = typescriptMaps(packageDir)
    const dist = join(packageDir, "dist")
    let unloaded = 0
    let unverified = 0
    for (const chunk of measured ? await readdir(dist, {recursive: true}) : []) {
        const file = join(dist, chunk)
        if (!file.endsWith(".js") || !existsSync(`${file}.map`)) continue
        const bundle = bundleMap(file, packageDir, counted, typescript)
        if (!bundle.code.size) continue
        unverified += bundle.unverified
        const converter = v8ToIstanbul(
            file,
            0,
            {
                source: await readFile(file, "utf8"),
                originalSource: "",
                sourceMap: {sourcemap: bundle.map},
            },
            (path) => !counted(path)
        )
        await converter.load()
        const functions = executed.get(file)
        if (functions) converter.applyCoverage(functions)
        else unloaded++
        for (const [path, data] of Object.entries(converter.toIstanbul()))
            if (counted(path))
                coverage.addFileCoverage(
                    bundleStatements(
                        data,
                        bundle.text.get(path) ?? "",
                        bundle.code.get(path) ?? new Set(),
                        !!functions
                    )
                )
    }
    const rows = []
    if (coverage.files().length) {
        const total = coverage.getCoverageSummary().toJSON()
        const files = Object.fromEntries(
            coverage
                .files()
                .map((path) => [path, coverage.fileCoverageFor(path).toSummary().toJSON()])
        )
        await writeFile(
            join(raw, "coverage-summary.json"),
            JSON.stringify({total, ...files}, null, 2)
        )
        rows.push([`${name}/src`, ...metrics.map((metric) => ratio(total[metric]))])
    }
    const notes = measured && [
        `V8 coverage from ${measured} tests, mapped through the production source maps`,
        "and continued to the TypeScript by re-emitting each module. Statements are",
        "Istanbul's statement kinds that emit bundle code; functions and branches are",
        "V8's named functions and blocks, as v8-to-istanbul converts them.",
        unloaded
            ? `The statements of ${unloaded} lazy chunk(s) that never loaded count as uncovered.`
            : "",
        unverified ? `${unverified} module(s) could not be re-emitted and keep ES5 lines.` : "",
    ]
    await publish(
        [
            `### ${name} production journeys`,
            resultsTable("Journeys", junit),
            notes ? notes.filter(Boolean).join(" ") : "",
            rows.length
                ? table(["Source", "Lines", "Statements", "Functions", "Branches"], rows)
                : "No journey coverage was recorded; set `STEP_UI_JOURNEY_COVERAGE=1`.",
        ]
            .filter(Boolean)
            .join("\n\n")
    )
}

const [mode, target, title] = process.argv.slice(2)
if (mode === "stories" && target) await stories(target)
else if (mode === "journeys" && target) await journeys(target)
else if (mode === "results" && target) await results(target, title ?? basename(target))
else {
    process.stderr.write(
        "Usage: summary.mts stories|journeys <package-dir> | results <junit.xml> [title]\n"
    )
    process.exitCode = 2
}
