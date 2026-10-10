// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The problems an editor shows beside a monitoring document, wherever they
 * came from: the YAML parser, sequent-core's policy run in the browser, or
 * Harvest's validation and test render. Each is placed on the line its path
 * names, so the list and the editor's gutter agree.
 */

import {isMap, isScalar, isSeq, parseDocument, type Node} from "yaml"
import {EMonitoringProblemSeverity, type IMonitoringProblem} from "../editor/types"
import {EYamlParseStatus, parsePath, type TYamlParseResult} from "./yamlPatch"

export {normalizeProblems} from "./problems"

export enum EDiagnosticOrigin {
    /** The text is not YAML. */
    SYNTAX = "SYNTAX",
    /** sequent-core's policy, run in the browser on every keystroke. */
    LOCAL = "LOCAL",
    /** Harvest: the policy again, dbt Charts' checks and a test render. */
    SERVER = "SERVER",
}

export interface IEditorDiagnostic extends IMonitoringProblem {
    origin: EDiagnosticOrigin
    /** Character offsets of the text the problem is about. */
    from: number
    to: number
    /** 1-based line of `from`. */
    line: number
}

/** Paths from checks across documents start with the set they are in: `widgets.<key>.`. */
const SET_PREFIXES = new Set(["widgets", "dashboards", "themes"])
/** The key of a widget's dbt Charts document. */
const CHART = "chart"

const lineAt = (text: string, offset: number) => text.slice(0, offset).split("\n").length

interface IRange {
    from: number
    to: number
}

const rangeOf = (node: Node | null | undefined): IRange | undefined =>
    node?.range ? {from: node.range[0], to: node.range[1]} : undefined

/** The range of the deepest node of `segments` found; `undefined` when not even the first is. */
const walk = (root: Node | null, segments: Array<string | number>): IRange | undefined => {
    let node: Node | null | undefined = root
    let found: IRange | undefined
    for (const segment of segments) {
        if (isMap(node)) {
            const pair = node.items.find((item) => {
                const key = isScalar(item.key) ? item.key.value : item.key
                return String(key) === String(segment)
            })
            if (!pair) break
            const keyRange = rangeOf(pair.key as Node)
            const valueRange = rangeOf(pair.value as Node)
            found = keyRange
                ? {from: keyRange.from, to: valueRange?.to ?? keyRange.to}
                : (valueRange ?? found)
            node = pair.value as Node
        } else if (isSeq(node) && typeof segment === "number") {
            const item = node.items[segment] as Node | undefined
            if (!item) break
            found = rangeOf(item) ?? found
            node = item
        } else {
            break
        }
    }
    return found
}

/**
 * `segments` as a path from the document's root. A check across documents
 * puts the set it is in first (`widgets.<key>.`); dbt Charts' checks of a
 * widget name a path inside its `chart`.
 */
const withinDocument = (
    root: Node | null,
    segments: Array<string | number>
): Array<string | number> => {
    if (!isMap(root) || root.has(segments[0])) return segments
    if (SET_PREFIXES.has(String(segments[0]))) return segments.slice(2)
    const chart = root.get(CHART, true)
    return isMap(chart) && chart.has(segments[0]) ? [CHART, ...segments] : segments
}

/**
 * Where in `text` the policy's `path` points; the start of the document when
 * no part of it is there, as for a check of another document.
 */
export const locatePath = (text: string, path: string): IRange & {line: number} => {
    const start = {from: 0, to: 0, line: 1}
    const segments = parsePath(path)
    if (!segments.length) return start
    const document = parseDocument(text)
    if (document.errors.length) return start
    const root = document.contents as Node | null
    const range = walk(root, withinDocument(root, segments))
    if (!range) return start
    const to = trimTrailing(text, range.from, range.to)
    return {from: range.from, to, line: lineAt(text, range.from)}
}

/** A node's range runs through the line break after it; the editor marks the text only. */
const trimTrailing = (text: string, from: number, to: number) => {
    let end = Math.min(to, text.length)
    while (end > from && /\s/.test(text[end - 1])) end -= 1
    return end
}

const sameProblem = (left: IMonitoringProblem, right: IMonitoringProblem) =>
    left.severity === right.severity &&
    left.code === right.code &&
    left.path === right.path &&
    left.message === right.message

export interface IDiagnosticSources {
    local: IMonitoringProblem[]
    server: IMonitoringProblem[]
}

/**
 * Everything to show for `text`. While the YAML does not parse only the
 * syntax errors are shown: the other checks would all say the same.
 */
export const editorDiagnostics = (
    text: string,
    parsed: TYamlParseResult,
    {local, server}: IDiagnosticSources
): IEditorDiagnostic[] => {
    if (parsed.status === EYamlParseStatus.SYNTAX_ERROR) {
        return parsed.errors.map((error) => ({
            severity: EMonitoringProblemSeverity.ERROR,
            code: "unreadable",
            path: "",
            message: error.message,
            origin: EDiagnosticOrigin.SYNTAX,
            from: error.from,
            to: Math.max(error.to, error.from),
            line: error.line,
        }))
    }
    const seen: IMonitoringProblem[] = []
    const place = (origin: EDiagnosticOrigin) => (problem: IMonitoringProblem) => {
        if (seen.some((other) => sameProblem(other, problem))) return []
        seen.push(problem)
        return [{...problem, origin, ...locatePath(text, problem.path)}]
    }
    return [
        ...local.flatMap(place(EDiagnosticOrigin.LOCAL)),
        ...server.flatMap(place(EDiagnosticOrigin.SERVER)),
    ]
}

export const countBySeverity = (problems: ReadonlyArray<IMonitoringProblem>) => ({
    errors: problems.filter((problem) => problem.severity === EMonitoringProblemSeverity.ERROR)
        .length,
    warnings: problems.filter((problem) => problem.severity === EMonitoringProblemSeverity.WARNING)
        .length,
})
