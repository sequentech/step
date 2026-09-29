// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {EMonitoringProblemSeverity} from "../editor/types"
import {
    EDiagnosticOrigin,
    countBySeverity,
    editorDiagnostics,
    locatePath,
    normalizeProblems,
} from "./diagnostics"
import {EYamlParseStatus, parseYamlText} from "./yamlPatch"

const WIDGET = `id: turnout
title: Turnout
query:
  template: by_group
  group_by: {selector: breakdown}
chart:
  charts:
    bars:
      type: bar
      query: data
layout:
  - {widget: a, width: 6}
  - {widget: b, width: 14}
`

describe("normalizeProblems", () => {
    it("accepts sequent-core's lower-case severities and Harvest's upper-case ones", () => {
        expect(
            normalizeProblems([
                {severity: "error", code: "forbidden_key", path: "a", message: "m"},
                {severity: "WARNING", code: "unused_selector", path: "", message: "w"},
            ]).map((problem) => problem.severity)
        ).toEqual([EMonitoringProblemSeverity.ERROR, EMonitoringProblemSeverity.WARNING])
    })

    it("drops entries that are not problems and treats an unknown severity as an error", () => {
        expect(normalizeProblems([null, 3, {message: "only"}])).toEqual([
            {
                severity: EMonitoringProblemSeverity.ERROR,
                code: "",
                path: "",
                message: "only",
                engine_code: undefined,
            },
        ])
        expect(normalizeProblems(undefined)).toEqual([])
    })
})

describe("locatePath", () => {
    it("points at the key and value a path names", () => {
        const located = locatePath(WIDGET, "query.group_by")
        expect(WIDGET.slice(located.from, located.to)).toBe("group_by: {selector: breakdown}")
        expect(located.line).toBe(5)
    })

    it("follows indexes into sequences", () => {
        const located = locatePath(WIDGET, "layout[1].width")
        expect(WIDGET.slice(located.from, located.to)).toBe("width: 14")
        expect(located.line).toBe(13)
    })

    it("falls back to the deepest part of the path that exists", () => {
        const located = locatePath(WIDGET, "chart.charts.bars.style.axis")
        expect(WIDGET.slice(located.from, located.to)).toMatch(/^bars:/)
    })

    it("strips the set prefix validate_set puts in front of a widget's path", () => {
        const located = locatePath(WIDGET, "widgets.turnout.query.template")
        expect(WIDGET.slice(located.from, located.to)).toBe("template: by_group")
    })

    it("points at the start of the document for an empty path", () => {
        expect(locatePath(WIDGET, "")).toEqual({from: 0, to: 0, line: 1})
    })
})

describe("editorDiagnostics", () => {
    const problem = (path: string, message: string) => ({
        severity: EMonitoringProblemSeverity.ERROR,
        code: "invalid_value",
        path,
        message,
    })

    it("shows only the syntax errors while the YAML does not parse", () => {
        const text = "id: [a\n"
        const parsed = parseYamlText(text)
        expect(parsed.status).toBe(EYamlParseStatus.SYNTAX_ERROR)
        const diagnostics = editorDiagnostics(text, parsed, {
            local: [problem("id", "local")],
            server: [problem("id", "server")],
        })
        expect(diagnostics.every((item) => item.origin === EDiagnosticOrigin.SYNTAX)).toBe(true)
        expect(diagnostics.length).toBeGreaterThan(0)
    })

    it("merges local and server problems, the same problem once", () => {
        const parsed = parseYamlText(WIDGET)
        const diagnostics = editorDiagnostics(WIDGET, parsed, {
            local: [problem("layout[1].width", "too wide")],
            server: [
                problem("layout[1].width", "too wide"),
                {...problem("chart", "chart refused"), engine_code: "ERR-1"},
            ],
        })
        expect(diagnostics.map((item) => [item.origin, item.message, item.line])).toEqual([
            [EDiagnosticOrigin.LOCAL, "too wide", 13],
            [EDiagnosticOrigin.SERVER, "chart refused", 6],
        ])
    })

    it("counts errors and warnings", () => {
        const parsed = parseYamlText(WIDGET)
        const diagnostics = editorDiagnostics(WIDGET, parsed, {
            local: [
                problem("id", "a"),
                {...problem("title", "b"), severity: EMonitoringProblemSeverity.WARNING},
            ],
            server: [],
        })
        expect(countBySeverity(diagnostics)).toEqual({errors: 1, warnings: 1})
    })
})
