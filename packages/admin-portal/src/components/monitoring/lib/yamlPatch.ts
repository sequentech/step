// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Editing monitoring YAML the way its author would: a form changes one value
 * and every comment, blank line and flow mapping around it stays where it was.
 *
 * The editor's YAML text is the single source of truth; the forms never keep
 * a copy. Each change parses the text, edits the document tree and prints it
 * again, so the YAML tab always shows exactly what will be saved.
 */

import {
    Document,
    isCollection,
    isMap,
    isScalar,
    isSeq,
    parseDocument,
    stringify,
    type Node,
} from "yaml"

export type TYamlPath = ReadonlyArray<string | number>

export enum EYamlParseStatus {
    OK = "OK",
    SYNTAX_ERROR = "SYNTAX_ERROR",
}

export interface IYamlSyntaxError {
    message: string
    /** 1-based line and column of the start of the error. */
    line: number
    column: number
    /** Character offsets into the text. */
    from: number
    to: number
}

export type TYamlParseResult =
    | {status: EYamlParseStatus.OK; document: Document.Parsed; value: unknown}
    | {status: EYamlParseStatus.SYNTAX_ERROR; errors: IYamlSyntaxError[]}

/** Printing options that change nothing the author did not change. */
const PRINT = {lineWidth: 0, minContentWidth: 0, flowCollectionPadding: false} as const

export const parseYamlText = (text: string): TYamlParseResult => {
    const document = parseDocument(text, {keepSourceTokens: false, prettyErrors: true})
    if (document.errors.length) {
        return {
            status: EYamlParseStatus.SYNTAX_ERROR,
            errors: document.errors.map((error) => {
                const [from, to] = error.pos
                const start = error.linePos?.[0]
                return {
                    message: error.message.split("\n")[0],
                    line: start?.line ?? 1,
                    column: start?.col ?? 1,
                    from: Math.max(0, Math.min(from, text.length)),
                    to: Math.max(0, Math.min(Math.max(to, from), text.length)),
                }
            }),
        }
    }
    return {status: EYamlParseStatus.OK, document, value: document.toJS()}
}

/**
 * `chart.charts.bars[0].query` → `["chart", "charts", "bars", 0, "query"]`,
 * the path format sequent-core's policy reports problems with.
 */
export const parsePath = (path: string): Array<string | number> => {
    const segments: Array<string | number> = []
    for (const part of path.split(".")) {
        if (!part) continue
        const match = /^([^[\]]*)((?:\[\d+\])*)$/.exec(part)
        if (!match) {
            segments.push(part)
            continue
        }
        if (match[1]) segments.push(match[1])
        for (const index of match[2].match(/\d+/g) ?? []) {
            segments.push(Number(index))
        }
    }
    return segments
}

const documentOf = (text: string): Document.Parsed => {
    const parsed = parseYamlText(text)
    if (parsed.status !== EYamlParseStatus.OK) {
        throw new Error(`The YAML has a syntax error: ${parsed.errors[0]?.message ?? ""}`)
    }
    return parsed.document
}

const print = (document: Document): string => document.toString(PRINT)

export const getIn = (text: string, path: TYamlPath): unknown => {
    const parsed = parseYamlText(text)
    if (parsed.status !== EYamlParseStatus.OK) return undefined
    const node = parsed.document.getIn(path, true) as Node | undefined
    if (node === undefined || node === null) return undefined
    if (isScalar(node)) return node.toJSON()
    return (node as {toJSON?: () => unknown}).toJSON?.() ?? node
}

const isPrimitive = (value: unknown): value is string | number | boolean | null =>
    value === null || ["string", "number", "boolean"].includes(typeof value)

/**
 * Whether a scalar's quotes were needed by its value (`""`, `"Turnout "`, a
 * number) rather than chosen by the author. A value typed one key at a time
 * passes through such states, and must not keep their quotes.
 */
const styleWasForced = (value: unknown) =>
    typeof value !== "string" || /^["']/.test(stringify(value))

/**
 * Sets the value at `path`, creating missing mappings on the way. A scalar
 * replaced by a scalar keeps its node, so its trailing comment stays with it.
 * `undefined` removes the key.
 */
export const setIn = (text: string, path: TYamlPath, value: unknown): string => {
    if (value === undefined) return deleteIn(text, path)
    const document = documentOf(text)
    const existing = document.getIn(path, true)
    if (isScalar(existing) && isPrimitive(value)) {
        if (existing.value === value) return text
        if (styleWasForced(existing.value)) existing.type = undefined
        existing.value = value
    } else if (document.contents === null || !isCollection(document.contents)) {
        const fresh = new Document({})
        fresh.commentBefore = document.commentBefore
        fresh.setIn(path, value)
        return print(fresh)
    } else {
        document.setIn(path, value)
    }
    return print(document)
}

export const deleteIn = (text: string, path: TYamlPath): string => {
    const document = documentOf(text)
    if (!document.hasIn(path)) return text
    document.deleteIn(path)
    return print(document)
}

/**
 * Moves item `from` of the sequence or mapping at `path` to position `to`,
 * with its comments. Mapping order matters in monitoring YAML: selectors show
 * in the order they are declared.
 */
export const moveIn = (text: string, path: TYamlPath, from: number, to: number): string => {
    const document = documentOf(text)
    const node = path.length ? document.getIn(path, true) : document.contents
    if (!isSeq(node) && !isMap(node)) return text
    const items = node.items as unknown[]
    if (from === to || from < 0 || to < 0 || from >= items.length || to >= items.length) {
        return text
    }
    const [item] = items.splice(from, 1)
    items.splice(to, 0, item)
    return print(document)
}

/**
 * Inserts `value` at `index` of the sequence at `path` (past the end appends),
 * creating the sequence when it is missing. A flow sequence stays flow, a
 * block one block, like the items already in it.
 */
export const insertIn = (text: string, path: TYamlPath, index: number, value: unknown): string => {
    const document = documentOf(text)
    const node = document.getIn(path, true)
    if (node === undefined || node === null) return setIn(text, path, [value])
    if (!isSeq(node)) return text
    const item = document.createNode(value)
    const first = node.items[0]
    if (isCollection(item) && isCollection(first)) item.flow = first.flow
    node.items.splice(Math.max(0, Math.min(index, node.items.length)), 0, item)
    return print(document)
}

/** Renames key `from` of the mapping at `path` to `to`, keeping its position and value. */
export const renameKey = (text: string, path: TYamlPath, from: string, to: string): string => {
    const document = documentOf(text)
    const node = path.length ? document.getIn(path, true) : document.contents
    if (!isMap(node) || from === to || !to || node.has(to)) return text
    const pair = node.items.find((item) => {
        const key = isScalar(item.key) ? item.key.value : item.key
        return key === from
    })
    if (!pair) return text
    if (isScalar(pair.key)) {
        pair.key.value = to
    } else {
        pair.key = document.createNode(to)
    }
    return print(document)
}
