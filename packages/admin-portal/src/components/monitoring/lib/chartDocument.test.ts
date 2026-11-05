/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EColorScheme, MIN_RENDER_WIDTH_PX, WIDTH_BUCKET_PX} from "../types"
import {
    CHART_CSP,
    chartDocument,
    hashString,
    previewWidth,
    sanitizeSvg,
    widthBucket,
} from "./chartDocument"
import {CHART_FONT_CSS} from "./chartFonts"

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

describe("sanitizeSvg styles", () => {
    it("drops @import and every url() that leaves the document from a style element", () => {
        const clean = sanitizeSvg(
            svg(
                "<style>@import url(https://evil/x.css); rect{fill:url(https://evil/p)}" +
                    " circle{fill:url(#grad)} @import 'https://evil/y.css';</style><rect/>"
            )
        )
        expect(clean).not.toMatch(/evil|@import/i)
        expect(clean).toContain("fill:url(#grad)")
        expect(clean).toContain("<style>")
    })

    it("drops a url() that leaves the document from a style attribute", () => {
        const clean = sanitizeSvg(
            svg(
                '<rect style="fill:url(https://evil/p); stroke:red"/>' +
                    '<circle style="fill:url(#grad)"/>' +
                    "<ellipse style='fill:URL( \"https://evil/q\" )'/>"
            )
        )
        expect(clean).not.toMatch(/evil/i)
        expect(clean).toContain("stroke:red")
        expect(clean).toContain("fill:url(#grad)")
    })

    it("drops a style that hides a url() behind a CSS escape", () => {
        const clean = sanitizeSvg(
            svg('<style>rect{fill:u\\72l(https://evil/p)}</style><rect style="fill:\\75rl(x)"/>')
        )
        expect(clean).not.toMatch(/evil|72l|75rl/i)
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

    it("scales a chart wider than the frame down to it, with no sideways scroll", () => {
        const {html} = chartDocument({svg: svg("<rect/>"), colorScheme: EColorScheme.LIGHT})
        expect(html).toContain("svg { display: block; max-width: 100%; height: auto; }")
        expect(html).toContain("overflow-x: hidden")
    })

    it("reads the chart's size from the sanitized root", () => {
        const tall =
            '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 607.7" width="400">' +
            "<rect/></svg>"
        expect(chartDocument({svg: tall, colorScheme: EColorScheme.LIGHT}).size).toEqual({
            aspect: 607.7 / 400,
            width: 400,
        })
        // What the sanitizer takes off the root does not hide the size.
        const handled =
            '<svg xmlns="http://www.w3.org/2000/svg" onload="alert(1)" viewBox="0 0 200 100">' +
            "<rect/></svg>"
        expect(chartDocument({svg: handled, colorScheme: EColorScheme.LIGHT}).size).toEqual({
            aspect: 0.5,
        })
        expect(
            chartDocument({svg: svg("<rect/>"), colorScheme: EColorScheme.LIGHT}).size
        ).toBeNull()
    })

    it("gives the frame the engine's font as a data: URI the policy allows", () => {
        expect(CHART_FONT_CSS).toMatch(
            /^@font-face\{font-family:'Inter Variable';src:url\(data:font\/woff;base64,[A-Za-z0-9+/=]+\)/
        )
        const {html} = chartDocument({
            svg: svg("<rect/>"),
            colorScheme: EColorScheme.LIGHT,
            fontCss: CHART_FONT_CSS,
        })
        expect(html).toContain("<style>@font-face{font-family:'Inter Variable'")
        expect(CHART_CSP).toContain("font-src data:")
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
    it("rounds a width down to 40 px steps", () => {
        expect(widthBucket(419)).toBe(400)
        expect(widthBucket(440)).toBe(440)
        expect(widthBucket(1000)).toBe(1000)
    })

    it("never asks for less than the minimum render width", () => {
        // A phone's card is about 260 px wide: drawn at 360 and scaled down,
        // not drawn at 240 with every label cut and bars of no width.
        expect(MIN_RENDER_WIDTH_PX).toBe(360)
        expect(MIN_RENDER_WIDTH_PX % WIDTH_BUCKET_PX).toBe(0)
        expect(widthBucket(260)).toBe(360)
        expect(widthBucket(399)).toBe(360)
        expect(widthBucket(10)).toBe(360)
        expect(widthBucket(0)).toBe(360)
        expect(widthBucket(-5)).toBe(360)
    })
})

describe("previewWidth", () => {
    it("draws a preview at the width of the card it was opened from", () => {
        expect(previewWidth(880, 600)).toBe(880)
    })

    it("draws one opened from elsewhere at the width of the pane it shows in", () => {
        expect(previewWidth(undefined, 613)).toBe(600)
    })

    it("draws no preview narrower than a card is ever drawn, whatever was measured", () => {
        expect(previewWidth(undefined, null)).toBe(MIN_RENDER_WIDTH_PX)
        expect(previewWidth(undefined, 200)).toBe(MIN_RENDER_WIDTH_PX)
        expect(previewWidth(250, 600)).toBe(MIN_RENDER_WIDTH_PX)
    })
})
