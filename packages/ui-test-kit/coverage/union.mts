// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// The per-file union of line coverage from several test layers. Each layer
// gives, per source line, the number of times it ran; a line is covered when
// any layer ran it. Layers may disagree on which lines hold statements, so a
// file's lines are every line any layer reports.

/** Hits per source line of one file. */
export type LineHits = Map<number, number>
/** Line hits per source file, keyed by the package-relative path (`src/...`). */
export type LayerLines = Map<string, LineHits>

interface IstanbulFile {
    path?: string
    statementMap: Record<string, {start: {line: number}}>
    s: Record<string, number>
}

/** Maps a report's path to the package-relative path, or drops it. */
export type Relative = (path: string) => string | undefined

// The sources the package's Jest and Storybook coverage inventories count.
const EXCLUDED = [/\.d\.ts$/, /\.test\.tsx?$/, /\.stories\.tsx?$/, /\/__stories__\//]

/** Relative paths under `<package>/src/`, with the inventories' exclusions. */
export function packageSources(packageName: string): Relative {
    const marker = `/${packageName}/src/`
    return (path) => {
        const normal = path.replace(/\\/g, "/")
        const index = normal.lastIndexOf(marker)
        const relative = normal.startsWith("src/")
            ? normal
            : index >= 0
              ? normal.slice(index + packageName.length + 2)
              : undefined
        if (!relative || !/\.tsx?$/.test(relative)) return
        return EXCLUDED.some((pattern) => pattern.test(relative)) ? undefined : relative
    }
}

function addHits(layer: LayerLines, file: string, line: number, hits: number) {
    let lines = layer.get(file)
    if (!lines) layer.set(file, (lines = new Map()))
    lines.set(line, Math.max(lines.get(line) ?? 0, hits))
}

/**
 * Line hits from an Istanbul `coverage-final.json`, by Istanbul's own rule: a
 * line's count is the highest count of the statements that start on it.
 */
export function istanbulLines(report: Record<string, IstanbulFile>, relative: Relative) {
    const layer: LayerLines = new Map()
    for (const [key, file] of Object.entries(report)) {
        const path = relative(file.path ?? key)
        if (!path) continue
        if (!layer.has(path)) layer.set(path, new Map())
        for (const [id, {start}] of Object.entries(file.statementMap))
            addHits(layer, path, start.line, file.s[id] ?? 0)
    }
    return layer
}

/** Line hits from the `SF` and `DA` records of an LCOV tracefile. */
export function lcovLines(tracefile: string, relative: Relative) {
    const layer: LayerLines = new Map()
    let path: string | undefined
    for (const record of tracefile.split(/\r?\n/)) {
        if (record.startsWith("SF:")) {
            path = relative(record.slice(3))
            if (path && !layer.has(path)) layer.set(path, new Map())
        } else if (record === "end_of_record") path = undefined
        else if (path && record.startsWith("DA:")) {
            const [line, hits] = record.slice(3).split(",").map(Number)
            if (Number.isInteger(line) && Number.isFinite(hits)) addHits(layer, path, line, hits)
        }
    }
    return layer
}

/** Several reports of one layer (for example, test shards) as one. */
export function mergeLayers(reports: LayerLines[]) {
    const merged: LayerLines = new Map()
    for (const report of reports)
        for (const [file, lines] of report) {
            if (!merged.has(file)) merged.set(file, new Map())
            for (const [line, hits] of lines) addHits(merged, file, line, hits)
        }
    return merged
}

export interface UnionFile {
    file: string
    /** Lines any layer reports for the file. */
    lines: number
    /** Lines at least one layer ran. */
    covered: number
    /** Per layer, how many of the file's lines it ran. */
    layers: Record<string, number>
    uncovered: number[]
}

/** The per-file union of the given layers, sorted by path. */
export function unionFiles(layers: Record<string, LayerLines>): UnionFile[] {
    const files = new Set(Object.values(layers).flatMap((layer) => [...layer.keys()]))
    return [...files].sort().map((file) => {
        const lines = new Set<number>()
        const covered = new Set<number>()
        const perLayer: Record<string, number> = {}
        for (const [name, layer] of Object.entries(layers)) {
            let ran = 0
            for (const [line, hits] of layer.get(file) ?? []) {
                lines.add(line)
                if (hits > 0) {
                    covered.add(line)
                    ran++
                }
            }
            perLayer[name] = ran
        }
        const uncovered = [...lines].filter((line) => !covered.has(line)).sort((a, b) => a - b)
        return {file, lines: lines.size, covered: covered.size, layers: perLayer, uncovered}
    })
}

/** `src/<dir>/<subdir>` for nested sources, `src/<dir>` or `src/<file>` otherwise. */
export function area(file: string) {
    const parts = file.split("/")
    return parts.slice(0, parts.length > 3 ? 3 : 2).join("/")
}

export interface UnionTotals {
    name: string
    files: number
    lines: number
    covered: number
    layers: Record<string, number>
}

export function totals(name: string, files: UnionFile[]): UnionTotals {
    const layers: Record<string, number> = {}
    for (const file of files)
        for (const [layer, ran] of Object.entries(file.layers))
            layers[layer] = (layers[layer] ?? 0) + ran
    return {
        name,
        files: files.length,
        lines: files.reduce((sum, file) => sum + file.lines, 0),
        covered: files.reduce((sum, file) => sum + file.covered, 0),
        layers,
    }
}

export function byArea(files: UnionFile[]): UnionTotals[] {
    const groups = new Map<string, UnionFile[]>()
    for (const file of files)
        groups.set(area(file.file), [...(groups.get(area(file.file)) ?? []), file])
    return [...groups]
        .sort(([a], [b]) => a.localeCompare(b))
        .map(([name, group]) => totals(name, group))
}

/** Sorted line numbers as compact ranges: `3-5, 9`. */
export function lineRanges(lines: number[]) {
    const ranges: string[] = []
    for (let index = 0; index < lines.length; ) {
        let end = index
        while (end + 1 < lines.length && lines[end + 1] === lines[end] + 1) end++
        ranges.push(end > index ? `${lines[index]}-${lines[end]}` : `${lines[index]}`)
        index = end + 1
    }
    return ranges.join(", ")
}
