// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import DOMPurify, {type DOMPurify as Purifier} from "dompurify"
import {EColorScheme, MIN_RENDER_WIDTH_PX, WIDTH_BUCKET_PX} from "../types"
import {chartSize, type ChartSize} from "./chartSize"

// A chart is drawn in `<iframe sandbox="" srcDoc>`: no script runs, and this
// policy, first in the head, lets the document load nothing but inline styles
// and data: fonts and images. The server sanitizes the SVG too; cleaning it
// again here keeps the frame safe whichever renderer produced it.

export const CHART_CSP =
    "default-src 'none'; style-src 'unsafe-inline'; font-src data:; img-src data:"

const HREF_ATTRIBUTES = new Set(["href", "xlink:href"])

const IMPORT_RULE = /@import[^;]*(;|$)/gi
/** `url(...)` whose target, quoted or not, is not a `#fragment` of the document. */
const OUTSIDE_URL = /url\(\s*(?!['"]?\s*#)[^)]*\)?/gi

/**
 * CSS the chart may keep: no `@import`, and `url()` only to the document's own
 * gradients, patterns and filters. A CSS escape could spell `url` or `@import`
 * so that no pattern sees it, so styles with one are dropped whole.
 */
export function sanitizeCss(css: string): string {
    if (css.includes("\\")) return ""
    return css.replace(IMPORT_RULE, "").replace(OUTSIDE_URL, "none")
}

let purifier: Purifier | undefined

function svgPurifier(): Purifier {
    if (purifier) return purifier
    // An instance of its own, so that its hook applies to charts only.
    const instance = DOMPurify(window)
    instance.addHook("uponSanitizeAttribute", (_node, data) => {
        if (HREF_ATTRIBUTES.has(data.attrName) && !data.attrValue.trim().startsWith("#")) {
            data.keepAttr = false
        }
        if (data.attrName === "style") data.attrValue = sanitizeCss(data.attrValue)
    })
    instance.addHook("uponSanitizeElement", (node, data) => {
        if (data.tagName === "style" && node.textContent) {
            node.textContent = sanitizeCss(node.textContent)
        }
    })
    purifier = instance
    return instance
}

export function sanitizeSvg(svg: string): string {
    return svgPurifier().sanitize(svg, {
        USE_PROFILES: {svg: true, svgFilters: true},
        // Reused glyphs and markers; the hook keeps only in-document references.
        ADD_TAGS: ["use"],
        FORBID_TAGS: ["script", "meta", "link", "foreignObject", "a", "iframe"],
        FORBID_ATTR: ["srcset", "ping"],
    })
}

/** FNV-1a: cheap, stable, enough to tell two documents apart. */
export function hashString(text: string): string {
    let hash = 0x811c9dc5
    for (let index = 0; index < text.length; index++) {
        hash ^= text.charCodeAt(index)
        hash = Math.imul(hash, 0x01000193)
    }
    return (hash >>> 0).toString(16).padStart(8, "0")
}

/**
 * The width a render is requested at: whole 40 px steps, never below
 * {@link MIN_RENDER_WIDTH_PX}. A frame narrower than that shrinks the chart
 * to fit (`max-width: 100%; height: auto`).
 */
export function widthBucket(width: number): number {
    const steps = Math.floor(Math.max(0, width) / WIDTH_BUCKET_PX)
    return Math.max(MIN_RENDER_WIDTH_PX, steps * WIDTH_BUCKET_PX)
}

export interface ChartDocument {
    html: string
    hash: string
    /** The sanitized chart's shape, which sizes the frame. */
    size: ChartSize | null
}

export function chartDocument({
    svg,
    colorScheme,
    fontCss = "",
}: {
    svg: string
    colorScheme: EColorScheme
    /** `@font-face` rules with data: URIs, built once for every frame. */
    fontCss?: string
}): ChartDocument {
    const clean = sanitizeSvg(svg)
    const scheme = colorScheme === EColorScheme.DARK ? "dark" : "light"
    const html =
        "<!doctype html><html><head>" +
        `<meta http-equiv="Content-Security-Policy" content="${CHART_CSP}">` +
        '<meta charset="utf-8">' +
        "<style>" +
        fontCss +
        `:root { color-scheme: ${scheme}; }` +
        "html, body { margin: 0; height: 100%; background: transparent; }" +
        "body { overflow-y: auto; overflow-x: hidden; }" +
        "svg { display: block; max-width: 100%; height: auto; }" +
        "</style></head><body>" +
        clean +
        "</body></html>"
    return {html, hash: hashString(html), size: chartSize(clean)}
}
