/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EColorScheme} from "../types"
import {chartDocument, sanitizeSvg} from "./chartDocument"
import {CHART_MAX_HEIGHT_FACTOR, frameHeight} from "./chartSize"

const BUCKETS = 720
const TABLE_ROWS = 150
const ROW_PX = 24

/** A bar per bucket, labelled, and a table of rows under it, like the renderer draws. */
function chartBody(bars: number, rows: number, offsetY: number): string {
    const parts: string[] = []
    for (let bucket = 0; bucket < bars; bucket++) {
        const height = (bucket * 37) % 180
        parts.push(
            `<rect x="${bucket}" y="${200 - height}" width="0.8" height="${height}" ` +
                `fill="#3b6ea5" stroke="none"><title>${bucket}:00 · ${height * 11} voted</title></rect>`,
            `<text x="${bucket}" y="215" font-size="9" text-anchor="middle" ` +
                `fill="#555">${bucket % 24}h</text>`
        )
    }
    for (let row = 0; row < rows; row++) {
        const y = offsetY + row * ROW_PX
        parts.push(
            `<path d="M0 ${y}H720" stroke="#ddd" stroke-width="1"/>`,
            `<text x="8" y="${y + 16}" font-size="12" fill="#222">Post ${row + 1}</text>`,
            `<text x="712" y="${y + 16}" font-size="12" text-anchor="end" ` +
                `fill="#222">${(row * 1234).toLocaleString("en")}</text>`
        )
    }
    return parts.join("")
}

/** A chart of about `bytes`: whole charts stacked until it is that big. */
function bigSvg(bytes: number): {svg: string; rects: number; height: number} {
    const panelHeight = 240 + TABLE_ROWS * ROW_PX
    const groups: string[] = []
    let length = 0
    while (length < bytes) {
        const group =
            `<g transform="translate(0 ${groups.length * panelHeight})">` +
            chartBody(BUCKETS, TABLE_ROWS, 240) +
            "</g>"
        groups.push(group)
        length += group.length
    }
    const height = groups.length * panelHeight
    const svg =
        `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 720 ${height}" width="720" ` +
        `height="${height}"><style>text{font-family:'Inter Variable',sans-serif}</style>` +
        groups.join("") +
        "</svg>"
    return {svg, rects: groups.length * BUCKETS, height}
}

const count = (text: string, pattern: RegExp) => text.match(pattern)?.length ?? 0

describe("a large chart", () => {
    it.each([2, 5])("sanitizes a %i MB chart whole", (megabytes) => {
        const {svg, rects, height} = bigSvg(megabytes * 1024 * 1024)
        const sanitizeStart = performance.now()
        const clean = sanitizeSvg(svg)
        const sanitizeMs = performance.now() - sanitizeStart
        const documentStart = performance.now()
        const {html, size} = chartDocument({svg, colorScheme: EColorScheme.LIGHT})
        const documentMs = performance.now() - documentStart
        process.stdout.write(
            `[scale] svg_bytes=${svg.length} sanitize_ms=${sanitizeMs.toFixed(0)} ` +
                `chart_document_ms=${documentMs.toFixed(0)}\n`
        )
        // Nothing of the drawing is lost on the way.
        expect(count(clean, /<rect /g)).toBe(rects)
        expect(count(html, /<rect /g)).toBe(rects)
        expect(size).toEqual({aspect: height / 720, width: 720})
    })

    it("keeps a frame for a chart tens of thousands of px tall to twice the widget height", () => {
        const {svg, height} = bigSvg(5 * 1024 * 1024)
        // The declared size is far past any screen.
        expect(height).toBeGreaterThan(10_000)
        const {size} = chartDocument({svg, colorScheme: EColorScheme.LIGHT})
        expect(frameHeight(size, 720, 320)).toBe(CHART_MAX_HEIGHT_FACTOR * 320)
        // Narrowed to a phone's card, still capped: the frame scrolls.
        expect(frameHeight(size, 260, 320)).toBe(640)
    })
})
