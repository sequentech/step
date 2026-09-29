// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    deleteIn,
    getIn,
    insertIn,
    moveIn,
    parsePath,
    parseYamlText,
    renameKey,
    setIn,
    EYamlParseStatus,
} from "./yamlPatch"

const WIDGET = `# Turnout by group, as the preset ships it.
id: turnout-by-group
title: Turnout by group # shown on the card
source: voter_turnout
selectors:
  breakdown:
    label: Breakdown
    options: {sex: Sex, age_band: Age}
    default: age_band # the preset's choice
  measure:
    label: Show
    options: {voted_reg: Voted of registered}
    default: voted_reg
query:
  template: by_group
  group_by: {selector: breakdown}
layout:
  - {widget: a, width: 6}
  - {widget: b, width: 6}
  # the last one spans the grid
  - {widget: c, width: 12}
`

describe("parseYamlText", () => {
    it("reads a document and its value", () => {
        const parsed = parseYamlText(WIDGET)
        expect(parsed.status).toBe(EYamlParseStatus.OK)
        if (parsed.status !== EYamlParseStatus.OK) return
        expect((parsed.value as {source: string}).source).toBe("voter_turnout")
    })

    it("reports a syntax error with its line and column", () => {
        const parsed = parseYamlText("id: a\ntitle: [unclosed\n")
        expect(parsed.status).toBe(EYamlParseStatus.SYNTAX_ERROR)
        if (parsed.status !== EYamlParseStatus.SYNTAX_ERROR) return
        expect(parsed.errors.length).toBeGreaterThan(0)
        expect(parsed.errors[0].line).toBeGreaterThanOrEqual(2)
        expect(parsed.errors[0].from).toBeGreaterThanOrEqual(0)
    })

    it("treats an empty text as an empty document, not an error", () => {
        expect(parseYamlText("").status).toBe(EYamlParseStatus.OK)
    })
})

describe("parsePath", () => {
    it("splits dotted keys and indexes", () => {
        expect(parsePath("chart.charts.bars[0].query")).toEqual([
            "chart",
            "charts",
            "bars",
            0,
            "query",
        ])
        expect(parsePath("")).toEqual([])
        expect(parsePath("layout[2]")).toEqual(["layout", 2])
    })
})

describe("setIn", () => {
    it("changes a scalar and keeps every comment", () => {
        const next = setIn(WIDGET, ["selectors", "breakdown", "default"], "sex")
        expect(next).toContain("default: sex # the preset's choice")
        expect(next).toContain("# Turnout by group, as the preset ships it.")
        expect(next).toContain("title: Turnout by group # shown on the card")
        expect(next).toContain("# the last one spans the grid")
        expect(getIn(next, ["selectors", "breakdown", "default"])).toBe("sex")
    })

    it("changes only the line of the value it sets", () => {
        const next = setIn(WIDGET, ["query", "template"], "by_post")
        const before = WIDGET.split("\n")
        const changed = next.split("\n").filter((line, index) => line !== before[index])
        expect(changed).toEqual(["  template: by_post"])
    })

    it("does not keep the quotes an emptied field needed", () => {
        const emptied = setIn("title: Old # shown\n", ["title"], "")
        expect(emptied).toBe('title: "" # shown\n')
        expect(setIn(emptied, ["title"], "New")).toBe("title: New # shown\n")
        expect(setIn("title: 'kept'\n", ["title"], "still")).toBe("title: 'still'\n")
        expect(setIn("limit: 10\n", ["limit"], "12")).toBe('limit: "12"\n')
        // Typed one key at a time, a title passes through "Turnout " (quoted for its space).
        const typed = ["Turnout", "Turnout ", "Turnout b"].reduce(
            (text, title) => setIn(text, ["title"], title),
            "title: T\n"
        )
        expect(typed).toBe("title: Turnout b\n")
    })

    it("creates missing parents", () => {
        const next = setIn(WIDGET, ["query", "sort", "by"], "value")
        expect(getIn(next, ["query", "sort"])).toEqual({by: "value"})
    })

    it("replaces a structured value", () => {
        const next = setIn(WIDGET, ["query", "group_by"], "sex")
        expect(getIn(next, ["query", "group_by"])).toBe("sex")
        const back = setIn(next, ["query", "group_by"], {selector: "breakdown"})
        expect(getIn(back, ["query", "group_by"])).toEqual({selector: "breakdown"})
    })

    it("removes the key when the value is undefined", () => {
        const next = setIn(WIDGET, ["title"], undefined)
        expect(getIn(next, ["title"])).toBeUndefined()
        expect(next).not.toContain("title:")
    })

    it("refuses to patch text that is not YAML", () => {
        expect(() => setIn("id: [a\n", ["id"], "b")).toThrow()
    })

    it("fills an empty document", () => {
        expect(getIn(setIn("", ["id"], "x"), ["id"])).toBe("x")
    })
})

describe("deleteIn", () => {
    it("removes a key and leaves its siblings and comments", () => {
        const next = deleteIn(WIDGET, ["selectors", "measure"])
        expect(getIn(next, ["selectors", "measure"])).toBeUndefined()
        expect(getIn(next, ["selectors", "breakdown", "label"])).toBe("Breakdown")
        expect(next).toContain("# the preset's choice")
    })

    it("is a no-op for a missing path", () => {
        expect(deleteIn(WIDGET, ["nothing", "here"])).toBe(WIDGET)
    })
})

describe("moveIn", () => {
    it("moves a sequence item and keeps the comment of the one moved", () => {
        const next = moveIn(WIDGET, ["layout"], 2, 0)
        const layout = getIn(next, ["layout"]) as {widget: string}[]
        expect(layout.map((item) => item.widget)).toEqual(["c", "a", "b"])
        expect(next).toContain("# the last one spans the grid")
    })

    it("moves a mapping entry, which is how selector order is kept", () => {
        const next = moveIn(WIDGET, ["selectors"], 1, 0)
        expect(Object.keys(getIn(next, ["selectors"]) as object)).toEqual(["measure", "breakdown"])
    })

    it("ignores an index outside the collection", () => {
        expect(moveIn(WIDGET, ["layout"], 5, 0)).toBe(WIDGET)
        expect(moveIn(WIDGET, ["layout"], 0, -1)).toBe(WIDGET)
    })
})

describe("insertIn", () => {
    it("inserts into a sequence at a position, in the sequence's own style", () => {
        const next = insertIn(WIDGET, ["layout"], 1, {widget: "d", width: 6})
        const layout = getIn(next, ["layout"]) as {widget: string}[]
        expect(layout.map((item) => item.widget)).toEqual(["a", "d", "b", "c"])
        expect(next).toContain("# the last one spans the grid")
    })

    it("appends past the end and creates a missing sequence", () => {
        const appended = insertIn(WIDGET, ["layout"], 99, {widget: "z", width: 12})
        expect((getIn(appended, ["layout"]) as {widget: string}[]).pop()?.widget).toBe("z")
        expect(getIn(insertIn("id: d\n", ["layout"], 0, {widget: "a"}), ["layout"])).toEqual([
            {widget: "a"},
        ])
    })
})

describe("renameKey", () => {
    it("renames a mapping key in place", () => {
        const next = renameKey(WIDGET, ["selectors"], "measure", "ratio")
        expect(Object.keys(getIn(next, ["selectors"]) as object)).toEqual(["breakdown", "ratio"])
        expect(getIn(next, ["selectors", "ratio", "label"])).toBe("Show")
    })

    it("refuses to overwrite an existing key", () => {
        expect(renameKey(WIDGET, ["selectors"], "measure", "breakdown")).toBe(WIDGET)
    })
})
