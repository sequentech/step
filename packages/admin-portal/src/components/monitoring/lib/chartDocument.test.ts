/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EColorScheme} from "../types"
import {CHART_CSP, chartDocument, hashString, sanitizeSvg, widthBucket} from "./chartDocument"

const svg = (inner: string) =>
    `<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50">${inner}</svg>`

describe("sanitizeSvg", () => {
    it("keeps the drawing", () => {
        const clean = sanitizeSvg(
            svg('<rect width="10" height="10" fill="#123456"/><text x="1">Hi</text>')
        )
        expect(clean).toContain("<rect")
        expect(clean).toContain("Hi")
    })

    it("strips scripts, handlers, foreign content and links out of the document", () => {
        const clean = sanitizeSvg(
            svg(
                '<script>alert(1)</script><rect onload="alert(2)"/>' +
                    "<foreignObject><div>x</div></foreignObject>" +
                    '<a href="https://example.com"><text>out</text></a>' +
                    '<use href="#glyph"/><image href="http://example.com/a.png"/>'
            )
        )
        expect(clean).not.toMatch(/script|onload|foreignObject|example\.com/i)
        expect(clean).toContain('href="#glyph"')
    })

    it("strips meta and link elements", () => {
        const clean = sanitizeSvg(
            svg('<meta http-equiv="refresh"/><link rel="stylesheet" href="x.css"/>')
        )
        expect(clean).not.toMatch(/meta|link/i)
    })
})

describe("chartDocument", () => {
    it("puts the content security policy first in the head", () => {
        const {html} = chartDocument({svg: svg("<rect/>"), colorScheme: EColorScheme.LIGHT})
        const head = html.slice(html.indexOf("<head>") + "<head>".length)
        expect(
            head.startsWith(`<meta http-equiv="Content-Security-Policy" content="${CHART_CSP}">`)
        ).toBe(true)
        expect(CHART_CSP).toBe(
            "default-src 'none'; style-src 'unsafe-inline'; font-src data:; img-src data:"
        )
        expect(html).toContain("overflow-y: auto")
        expect(html).toContain("<rect")
    })

    it("has a hash that changes only with the document", () => {
        const one = chartDocument({svg: svg("<rect/>"), colorScheme: EColorScheme.LIGHT})
        const same = chartDocument({svg: svg("<rect/>"), colorScheme: EColorScheme.LIGHT})
        const dark = chartDocument({svg: svg("<rect/>"), colorScheme: EColorScheme.DARK})
        expect(one.hash).toBe(same.hash)
        expect(one.hash).not.toBe(dark.hash)
        expect(hashString("a")).not.toBe(hashString("b"))
    })
})

describe("widthBucket", () => {
    it("rounds a width down to 40 px steps, never below one step", () => {
        expect(widthBucket(419)).toBe(400)
        expect(widthBucket(440)).toBe(440)
        expect(widthBucket(10)).toBe(40)
        expect(widthBucket(0)).toBe(40)
    })
})
