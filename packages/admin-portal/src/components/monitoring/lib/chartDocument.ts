// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import DOMPurify, {type DOMPurify as Purifier} from "dompurify"
import {EColorScheme, WIDTH_BUCKET_PX} from "../types"

// A chart is drawn in `<iframe sandbox="" srcDoc>`: no script runs, and this
// policy, first in the head, lets the document load nothing but inline styles
// and data: fonts and images. The server sanitizes the SVG too; cleaning it
// again here keeps the frame safe whichever renderer produced it.

export const CHART_CSP =
    "default-src 'none'; style-src 'unsafe-inline'; font-src data:; img-src data:"

const HREF_ATTRIBUTES = new Set(["href", "xlink:href"])

let purifier: Purifier | undefined

function svgPurifier(): Purifier {
    if (purifier) return purifier
    // An instance of its own, so that its hook applies to charts only.
    const instance = DOMPurify(window)
    instance.addHook("uponSanitizeAttribute", (_node, data) => {
        if (HREF_ATTRIBUTES.has(data.attrName) && !data.attrValue.trim().startsWith("#")) {
            data.keepAttr = false
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

/** The width a render is requested at: whole 40 px steps, at least one. */
export function widthBucket(width: number): number {
    const steps = Math.floor(Math.max(0, width) / WIDTH_BUCKET_PX)
    return Math.max(1, steps) * WIDTH_BUCKET_PX
}

export interface ChartDocument {
    html: string
    hash: string
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
        sanitizeSvg(svg) +
        "</body></html>"
    return {html, hash: hashString(html)}
}
