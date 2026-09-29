// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import DOMPurify from "dompurify"

/**
 * The document the editor's preview frame shows: the renderer's SVG,
 * stripped of anything that could run or reach the network, behind a CSP
 * that forbids both. The frame itself is `sandbox=""`.
 */
export const PREVIEW_CSP =
    "default-src 'none'; style-src 'unsafe-inline'; font-src data:; img-src data:"

const FORBIDDEN_TAGS = ["script", "meta", "link", "foreignObject", "iframe", "object", "embed"]

export const sanitizeChartSvg = (svg: string): string => {
    const purifier = DOMPurify()
    purifier.addHook("afterSanitizeAttributes", (node) => {
        for (const attribute of ["href", "xlink:href"]) {
            const value = node.getAttribute(attribute)
            if (value !== null && !value.startsWith("#")) node.removeAttribute(attribute)
        }
    })
    return purifier.sanitize(svg, {
        USE_PROFILES: {svg: true, svgFilters: true},
        FORBID_TAGS: FORBIDDEN_TAGS,
    })
}

export const previewDocument = (svg: string): string =>
    "<!doctype html><html><head>" +
    `<meta http-equiv="Content-Security-Policy" content="${PREVIEW_CSP}">` +
    "<style>html,body{margin:0;background:transparent}svg{display:block;max-width:100%;height:auto}</style>" +
    `</head><body>${sanitizeChartSvg(svg)}</body></html>`
