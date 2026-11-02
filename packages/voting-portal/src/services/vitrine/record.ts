// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the record viewer works out from a record's text (design board
// RecordViewer): its format, how it reads laid out and on one line, how much
// it holds and how big it is, and the colours of a line of JSON.

export enum ERecordFormat {
    JSON = "json",
    TEXT = "text",
}

export enum ECountKind {
    FIELDS = "fields",
    ITEMS = "items",
}

/** The media type a record is saved with. */
export const RECORD_MEDIA_TYPES: Record<ERecordFormat, string> = {
    [ERecordFormat.JSON]: "application/json",
    [ERecordFormat.TEXT]: "text/plain",
}

export interface IRecordCount {
    kind: ECountKind
    count: number
}

export interface IRecordText {
    format: ERecordFormat
    /** JSON laid out for reading, any other text as it is. */
    pretty: string
    /** On one line, for the preview. */
    compact: string
    /** The fields of a JSON object or the items of a JSON list. */
    count?: IRecordCount
    /** The size of the text it was given. */
    bytes: number
}

const JSON_INDENT = 2
const WHITESPACE = /\s+/g

const countOf = (value: unknown): IRecordCount | undefined => {
    if (Array.isArray(value)) return {kind: ECountKind.ITEMS, count: value.length}
    if (typeof value === "object" && value !== null) {
        return {kind: ECountKind.FIELDS, count: Object.keys(value).length}
    }
    return undefined
}

export const readRecordText = (content: string): IRecordText => {
    const bytes = new TextEncoder().encode(content).length
    let value: unknown
    try {
        value = JSON.parse(content)
    } catch {
        return {
            format: ERecordFormat.TEXT,
            pretty: content,
            compact: content.replace(WHITESPACE, " ").trim(),
            bytes,
        }
    }
    const count = countOf(value)
    return {
        format: ERecordFormat.JSON,
        pretty: JSON.stringify(value, null, JSON_INDENT),
        compact: JSON.stringify(value),
        ...(count ? {count} : {}),
        bytes,
    }
}

export enum EByteUnit {
    BYTES = "bytes",
    KILOBYTES = "kilobytes",
}

export interface IByteSize {
    unit: EByteUnit
    value: string
}

const KILOBYTE = 1024

const withOneDecimal = (value: number, language: string): string => {
    try {
        return new Intl.NumberFormat(language, {
            minimumFractionDigits: 1,
            maximumFractionDigits: 1,
        }).format(value)
    } catch {
        return value.toFixed(1)
    }
}

/** A size in bytes, or in kilobytes with one decimal from a kilobyte up. */
export const byteSizeOf = (bytes: number, language: string): IByteSize =>
    bytes < KILOBYTE
        ? {unit: EByteUnit.BYTES, value: String(bytes)}
        : {unit: EByteUnit.KILOBYTES, value: withOneDecimal(bytes / KILOBYTE, language)}

export enum ETokenKind {
    KEY = "key",
    STRING = "string",
    NUMBER = "number",
    LITERAL = "literal",
    PUNCTUATION = "punctuation",
    TEXT = "text",
}

export interface IToken {
    kind: ETokenKind
    text: string
}

// A string (and the colon of a key), a number, true, false or null, a bracket,
// brace, comma, a run of spaces, any other run, and last, any character left.
const TOKEN =
    /("(?:[^"\\]|\\.)*")(\s*:)?|(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)|\b(true|false|null)\b|([{}[\],])|(\s+)|([^\s"{}[\],]+)|([\s\S])/g

/** A line of JSON, cut into what the viewer colours alike. */
export const tokenizeLine = (line: string): IToken[] =>
    Array.from(line.matchAll(TOKEN)).flatMap((match): IToken[] => {
        const [text, string, colon, number, literal, punctuation] = match
        if (string !== undefined && colon !== undefined) {
            return [
                {kind: ETokenKind.KEY, text: string},
                {kind: ETokenKind.PUNCTUATION, text: colon},
            ]
        }
        if (string !== undefined) return [{kind: ETokenKind.STRING, text: string}]
        if (number !== undefined) return [{kind: ETokenKind.NUMBER, text: number}]
        if (literal !== undefined) return [{kind: ETokenKind.LITERAL, text: literal}]
        if (punctuation !== undefined) return [{kind: ETokenKind.PUNCTUATION, text: punctuation}]
        return [{kind: ETokenKind.TEXT, text}]
    })

/** The record's lines, each cut into what the viewer colours alike: JSON by
 * its syntax, any other text as one. */
export const recordLines = (record: IRecordText): IToken[][] =>
    record.pretty
        .split("\n")
        .map((line) =>
            record.format === ERecordFormat.JSON
                ? tokenizeLine(line)
                : [{kind: ETokenKind.TEXT, text: line}]
        )
