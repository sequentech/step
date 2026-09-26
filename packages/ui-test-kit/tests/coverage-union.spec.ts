// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect} from "@playwright/test"
import {execFile} from "node:child_process"
import {mkdir, mkdtemp, readFile, rm, writeFile} from "node:fs/promises"
import {tmpdir} from "node:os"
import {dirname, join, resolve} from "node:path"
import {promisify} from "node:util"
import {
    area,
    byArea,
    istanbulLines,
    lcovLines,
    lineRanges,
    mergeLayers,
    packageSources,
    unionFiles,
    type LayerLines,
} from "../coverage/union.mts"

const relative = packageSources("admin-portal")
const layer = (files: Record<string, Record<number, number>>): LayerLines =>
    new Map(
        Object.entries(files).map(([file, lines]) => [
            file,
            new Map(Object.entries(lines).map(([line, hits]) => [Number(line), hits])),
        ])
    )
const plain = (lines: LayerLines) =>
    Object.fromEntries([...lines].map(([file, hits]) => [file, Object.fromEntries(hits)]))

// Two statements on line 3 (0 and 2 hits) and one on line 5, as Istanbul writes them.
const istanbulFile = (path: string) => ({
    path,
    statementMap: {
        "0": {start: {line: 3, column: 0}, end: {line: 3, column: 9}},
        "1": {start: {line: 3, column: 10}, end: {line: 4, column: 2}},
        "2": {start: {line: 5, column: 0}, end: {line: 5, column: 4}},
    },
    s: {"0": 0, "1": 2, "2": 0},
})

test("package sources keep src TypeScript and drop tests, stories and declarations", () => {
    expect(relative("/__w/step/step/packages/admin-portal/src/App.tsx")).toBe("src/App.tsx")
    expect(relative("src/queries/GetUsers.ts")).toBe("src/queries/GetUsers.ts")
    expect(relative("C:\\step\\packages\\admin-portal\\src\\hooks\\a.ts")).toBe("src/hooks/a.ts")
    for (const excluded of [
        "/x/admin-portal/src/services/Password.test.ts",
        "/x/admin-portal/src/components/NoItem.stories.tsx",
        "/x/admin-portal/src/__stories__/helpers.tsx",
        "/x/admin-portal/src/react-app-env.d.ts",
        "/x/admin-portal/src/index.css",
        "/x/voting-portal/src/App.tsx",
        "/x/admin-portal/test/journeys/fixtures.ts",
    ])
        expect(relative(excluded), excluded).toBeUndefined()
})

test("Istanbul line hits take the highest count of the statements starting on a line", () => {
    const report = {
        "/w/packages/admin-portal/src/a.ts": istanbulFile("/w/packages/admin-portal/src/a.ts"),
        "/w/packages/admin-portal/src/a.test.ts": istanbulFile(
            "/w/packages/admin-portal/src/a.test.ts"
        ),
        "/w/packages/admin-portal/src/empty.ts": {
            path: "/w/packages/admin-portal/src/empty.ts",
            statementMap: {},
            s: {},
        },
    }
    expect(plain(istanbulLines(report, relative))).toEqual({
        "src/a.ts": {3: 2, 5: 0},
        "src/empty.ts": {},
    })
})

test("LCOV line hits read each file's DA records and ignore other records", () => {
    const tracefile = [
        "TN:",
        "SF:src/a.ts",
        "FN:1,f",
        "DA:3,0",
        "DA:4,7",
        "BRDA:4,0,0,1",
        "end_of_record",
        "SF:src/a.stories.tsx",
        "DA:1,1",
        "end_of_record",
        "SF:/abs/packages/admin-portal/src/b.tsx",
        "DA:9,1",
        "end_of_record",
        "",
    ].join("\n")
    expect(plain(lcovLines(tracefile, relative))).toEqual({
        "src/a.ts": {3: 0, 4: 7},
        "src/b.tsx": {9: 1},
    })
})

test("the union counts every reported line once and covers it when any layer ran it", () => {
    const jest = layer({"src/a.ts": {1: 0, 2: 1, 3: 0}})
    const journeys = layer({"src/a.ts": {3: 4, 4: 0}, "src/b.ts": {7: 1}})
    expect(unionFiles({jest, journeys})).toEqual([
        {file: "src/a.ts", lines: 4, covered: 2, layers: {jest: 1, journeys: 1}, uncovered: [1, 4]},
        {file: "src/b.ts", lines: 1, covered: 1, layers: {jest: 0, journeys: 1}, uncovered: []},
    ])
})

test("a file no layer ran keeps all its lines uncovered", () => {
    expect(unionFiles({jest: layer({"src/idle.ts": {2: 0, 3: 0}}), stories: layer({})})).toEqual([
        {
            file: "src/idle.ts",
            lines: 2,
            covered: 0,
            layers: {jest: 0, stories: 0},
            uncovered: [2, 3],
        },
    ])
})

test("shards of one layer merge line by line", () => {
    const merged = mergeLayers([
        layer({"src/a.ts": {1: 1, 2: 0}}),
        layer({"src/a.ts": {2: 3}, "src/b.ts": {5: 0}}),
    ])
    expect(plain(merged)).toEqual({"src/a.ts": {1: 1, 2: 3}, "src/b.ts": {5: 0}})
})

test("areas group nested sources by their second directory and totals add up", () => {
    expect(area("src/components/menu/items/ElectionEvents.tsx")).toBe("src/components/menu")
    expect(area("src/components/CustomMenu.tsx")).toBe("src/components")
    expect(area("src/App.tsx")).toBe("src/App.tsx")
    const files = unionFiles({
        jest: layer({
            "src/components/A.tsx": {1: 1, 2: 0},
            "src/components/menu/B.tsx": {1: 0},
            "src/components/menu/C.tsx": {4: 1, 5: 1},
        }),
    })
    expect(byArea(files)).toEqual([
        {name: "src/components", files: 1, lines: 2, covered: 1, layers: {jest: 1}},
        {name: "src/components/menu", files: 2, lines: 3, covered: 2, layers: {jest: 2}},
    ])
})

test("uncovered lines print as compact ranges", () => {
    expect(lineRanges([1, 2, 3, 5, 7, 8])).toBe("1-3, 5, 7-8")
    expect(lineRanges([4])).toBe("4")
    expect(lineRanges([])).toBe("")
})

test("the safety-net summary identifies missing layers and shards", async () => {
    const root = await mkdtemp(join(tmpdir(), "safety-net-"))
    try {
        const packageDir = join(root, "admin-portal")
        const shards = join(root, "journeys")
        const write = async (path: string, content: string) => {
            await mkdir(dirname(path), {recursive: true})
            await writeFile(path, content)
        }
        await write(join(packageDir, "package.json"), '{"name": "admin-portal"}')
        await write(
            join(root, "jest.json"),
            JSON.stringify({x: istanbulFile(join(packageDir, "src/a.ts"))})
        )
        // Two journey shards: each ran a different line of src/a.ts.
        await write(
            join(shards, "1/test-results/lcov.info"),
            "SF:src/a.ts\nDA:5,1\nend_of_record\n"
        )
        await write(
            join(shards, "2/test-results/lcov.info"),
            "SF:src/a.ts\nDA:6,0\nend_of_record\nSF:src/b/c/d.ts\nDA:1,1\nend_of_record\n"
        )
        const script = resolve(__dirname, "../coverage/summary.mts")
        const args = [
            "--experimental-strip-types",
            "--no-warnings",
            script,
            "safety-net",
            packageDir,
            "--layer",
            `jest=${join(root, "jest.json")}`,
            "--layer",
            `stories=${join(root, "missing")}`,
            "--layer",
            `journeys=${join(shards, "1")}`,
            "--layer",
            `journeys=${join(shards, "2")}`,
        ]
        const {stdout} = await promisify(execFile)(process.execPath, args, {
            env: {...process.env, GITHUB_STEP_SUMMARY: ""},
        })
        // src/a.ts: lines 3 (Jest ran it), 5 (shard 1) and 6 (no one); src/b/c/d.ts: line 1.
        expect(stdout).toContain(`No stories line data at \`${join(root, "missing")}\``)
        expect(stdout).not.toContain("Partial journeys")
        expect(stdout).toContain("| `src/a.ts` | 3 | 1 (33.3%) | n/a | 1 (33.3%) | 2 (66.7%) |")
        expect(stdout).toContain(
            "| **admin-portal/src** (2 files) | 4 | 1 (25.0%) | n/a | 2 (50.0%) | 3 (75.0%) |"
        )
        const report = JSON.parse(
            await readFile(join(packageDir, "test-results/safety-net/coverage-union.json"), "utf8")
        )
        expect(report.files).toEqual([
            {
                file: "src/a.ts",
                lines: 3,
                covered: 2,
                layers: {jest: 1, journeys: 1},
                uncovered: [6],
            },
            {
                file: "src/b/c/d.ts",
                lines: 1,
                covered: 1,
                layers: {jest: 0, journeys: 1},
                uncovered: [],
            },
        ])
        const markdown = await readFile(
            join(packageDir, "test-results/safety-net/coverage-union.md"),
            "utf8"
        )
        expect(markdown).toContain(
            "| `src/a.ts` | 3 | 1 (33.3%) | n/a | 1 (33.3%) | 2 (66.7%) | 6 |"
        )
        // A surviving shard must never make the journey layer appear complete.
        await rm(join(shards, "2"), {recursive: true})
        const partial = await promisify(execFile)(process.execPath, args, {
            env: {...process.env, GITHUB_STEP_SUMMARY: ""},
        })
        const note = `Partial journeys line data: no report at \`${join(shards, "2")}\`; its column includes only available reports.`
        expect(partial.stdout).toContain(note)
        const partialReport = JSON.parse(
            await readFile(join(packageDir, "test-results/safety-net/coverage-union.json"), "utf8")
        )
        expect(partialReport.notes).toContain(note)
        expect(partialReport.total).toMatchObject({files: 1, lines: 2, covered: 2})
        expect(
            await readFile(join(packageDir, "test-results/safety-net/coverage-union.md"), "utf8")
        ).toContain(note)
    } finally {
        await rm(root, {recursive: true, force: true})
    }
})
