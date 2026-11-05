// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the record viewer works out from a record's text (design board
// RecordViewer): its format, how it reads laid out and on one line, how much
// it holds and how big it is, and the colours of a line of JSON.

import {
    EByteUnit,
    ECountKind,
    ERecordFormat,
    ETokenKind,
    byteSizeOf,
    readRecordText,
    recordLines,
    tokenizeLine,
} from "./record"

describe("reading a record's text", () => {
    it("lays JSON out for reading and keeps it compact for the preview", () => {
        const record = readRecordText('{"example":true,"items":[1,2,3],"note":"Pass any JSON"}')
        expect(record.format).toBe(ERecordFormat.JSON)
        expect(record.pretty).toBe(
            '{\n  "example": true,\n  "items": [\n    1,\n    2,\n    3\n  ],\n  "note": "Pass any JSON"\n}'
        )
        expect(record.compact).toBe('{"example":true,"items":[1,2,3],"note":"Pass any JSON"}')
    })

    it("compacts JSON that was laid out already", () => {
        const record = readRecordText('{\n    "a": 1,\n    "b": [ 2, 3 ]\n}')
        expect(record.compact).toBe('{"a":1,"b":[2,3]}')
        expect(record.pretty).toBe('{\n  "a": 1,\n  "b": [\n    2,\n    3\n  ]\n}')
    })

    it("counts the fields of an object and the items of a list", () => {
        expect(readRecordText('{"a":1,"b":2,"c":3}').count).toEqual({
            kind: ECountKind.FIELDS,
            count: 3,
        })
        expect(readRecordText("[1,2]").count).toEqual({kind: ECountKind.ITEMS, count: 2})
        expect(readRecordText("{}").count).toEqual({kind: ECountKind.FIELDS, count: 0})
        expect(readRecordText("[]").count).toEqual({kind: ECountKind.ITEMS, count: 0})
    })

    it.each(["42", '"text"', "true", "null"])("counts nothing in the JSON value %s", (content) => {
        const record = readRecordText(content)
        expect(record.format).toBe(ERecordFormat.JSON)
        expect(record.count).toBeUndefined()
    })

    it("shows text that isn't JSON as it is, collapsed on one line for the preview", () => {
        const record = readRecordText("a ballot,\n   as   plain\ttext")
        expect(record.format).toBe(ERecordFormat.TEXT)
        expect(record.pretty).toBe("a ballot,\n   as   plain\ttext")
        expect(record.compact).toBe("a ballot, as plain text")
        expect(record.count).toBeUndefined()
    })

    it.each(['{"a": ', "{'a': 1}", ""])("reads %j, which is not JSON, as text", (content) => {
        const record = readRecordText(content)
        expect(record.format).toBe(ERecordFormat.TEXT)
        expect(record.pretty).toBe(content)
    })

    it("measures the text it was given, in bytes", () => {
        expect(readRecordText("").bytes).toBe(0)
        expect(readRecordText('{"a":1}').bytes).toBe(7)
        expect(readRecordText("é").bytes).toBe(2)
        expect(readRecordText('{\n  "a": 1\n}').bytes).toBe(12)
    })
})

describe("the size of a record", () => {
    it("is in bytes below a kilobyte", () => {
        expect(byteSizeOf(0, "en")).toEqual({unit: EByteUnit.BYTES, value: "0"})
        expect(byteSizeOf(269, "en")).toEqual({unit: EByteUnit.BYTES, value: "269"})
        expect(byteSizeOf(1023, "en")).toEqual({unit: EByteUnit.BYTES, value: "1023"})
    })

    it("is in kilobytes with one decimal from a kilobyte", () => {
        expect(byteSizeOf(1024, "en")).toEqual({unit: EByteUnit.KILOBYTES, value: "1.0"})
        expect(byteSizeOf(1536, "en")).toEqual({unit: EByteUnit.KILOBYTES, value: "1.5"})
        expect(byteSizeOf(12345, "en")).toEqual({unit: EByteUnit.KILOBYTES, value: "12.1"})
    })

    it("follows the language's decimal separator", () => {
        expect(byteSizeOf(1536, "es").value).toBe("1,5")
    })

    it("falls back to a point for a language it doesn't know", () => {
        expect(byteSizeOf(1536, "not a language").value).toBe("1.5")
    })
})

describe("the colours of a line of JSON", () => {
    const kinds = (line: string) => tokenizeLine(line).map((token) => [token.kind, token.text])

    it("tells a key from a string, and marks the punctuation between them", () => {
        expect(kinds('  "alpha": "a3f1",')).toEqual([
            [ETokenKind.TEXT, "  "],
            [ETokenKind.KEY, '"alpha"'],
            [ETokenKind.PUNCTUATION, ":"],
            [ETokenKind.TEXT, " "],
            [ETokenKind.STRING, '"a3f1"'],
            [ETokenKind.PUNCTUATION, ","],
        ])
    })

    it("marks numbers, and true, false and null", () => {
        expect(kinds("[1, -2.5, 3e10, true, false, null]")).toEqual([
            [ETokenKind.PUNCTUATION, "["],
            [ETokenKind.NUMBER, "1"],
            [ETokenKind.PUNCTUATION, ","],
            [ETokenKind.TEXT, " "],
            [ETokenKind.NUMBER, "-2.5"],
            [ETokenKind.PUNCTUATION, ","],
            [ETokenKind.TEXT, " "],
            [ETokenKind.NUMBER, "3e10"],
            [ETokenKind.PUNCTUATION, ","],
            [ETokenKind.TEXT, " "],
            [ETokenKind.LITERAL, "true"],
            [ETokenKind.PUNCTUATION, ","],
            [ETokenKind.TEXT, " "],
            [ETokenKind.LITERAL, "false"],
            [ETokenKind.PUNCTUATION, ","],
            [ETokenKind.TEXT, " "],
            [ETokenKind.LITERAL, "null"],
            [ETokenKind.PUNCTUATION, "]"],
        ])
    })

    it("keeps a string's quotes and escapes whole, and a colon inside it", () => {
        expect(kinds('"say \\"hi\\": now"')).toEqual([[ETokenKind.STRING, '"say \\"hi\\": now"']])
    })

    it.each(['  {"a":  tru, @#}', '"never closed', 'x "y', "-", "a\u0000b"])(
        "loses none of %j, whatever it is",
        (line) => {
            expect(
                tokenizeLine(line)
                    .map((token) => token.text)
                    .join("")
            ).toBe(line)
        }
    )

    it("leaves what it doesn't know as text", () => {
        expect(kinds("@#")).toEqual([[ETokenKind.TEXT, "@#"]])
        expect(kinds("tru")).toEqual([[ETokenKind.TEXT, "tru"]])
    })

    it("has nothing to show for an empty line", () => {
        expect(tokenizeLine("")).toEqual([])
    })
})

describe("the lines of a record", () => {
    it("cuts the lines of JSON by its syntax", () => {
        const lines = recordLines(readRecordText('{"a":[1,true]}'))
        expect(lines.map((line) => line.map((token) => token.text).join(""))).toEqual([
            "{",
            '  "a": [',
            "    1,",
            "    true",
            "  ]",
            "}",
        ])
        expect(lines[1].map((token) => token.kind)).toEqual([
            ETokenKind.TEXT,
            ETokenKind.KEY,
            ETokenKind.PUNCTUATION,
            ETokenKind.TEXT,
            ETokenKind.PUNCTUATION,
        ])
    })

    it("keeps each line of other text whole, even a line that looks like JSON", () => {
        const lines = recordLines(readRecordText('first\n  "key": 1\n\nlast'))
        expect(lines).toEqual([
            [{kind: ETokenKind.TEXT, text: "first"}],
            [{kind: ETokenKind.TEXT, text: '  "key": 1'}],
            [{kind: ETokenKind.TEXT, text: ""}],
            [{kind: ETokenKind.TEXT, text: "last"}],
        ])
    })

    it("has one empty line for an empty record", () => {
        expect(recordLines(readRecordText(""))).toEqual([[{kind: ETokenKind.TEXT, text: ""}]])
    })
})
