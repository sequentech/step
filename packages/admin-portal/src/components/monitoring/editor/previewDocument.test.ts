/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {CHART_FONT_CSS} from "../lib/chartFonts"
import {previewDocument} from "./previewDocument"

const SVG = '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><text>1</text></svg>'

describe("previewDocument", () => {
    it("carries the chart's font, as the dashboard's frame does", () => {
        expect(previewDocument(SVG)).toContain(CHART_FONT_CSS)
    })

    it("keeps the frame's policy: inline styles and data: fonts only", () => {
        expect(previewDocument(SVG)).toContain("font-src data:")
        expect(previewDocument(SVG)).not.toContain("<script")
    })
})
