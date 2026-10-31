// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {INTER_VARIABLE_LATIN_WOFF} from "../fonts/interVariableLatin"

/**
 * The chart engine names `Inter Variable` first and measures its text with it,
 * but the renderer strips its `@font-face` rules, which point at a server the
 * frame cannot reach. The face is supplied here as a data: URI, which the
 * frame's policy allows; text outside it falls back to the SVG's later families.
 * Built once, and the same string for every frame.
 */
export const CHART_FONT_CSS =
    "@font-face{font-family:'Inter Variable';" +
    `src:url(data:font/woff;base64,${INTER_VARIABLE_LATIN_WOFF}) format('woff');` +
    "font-weight:100 900;font-style:normal;font-display:block}"
