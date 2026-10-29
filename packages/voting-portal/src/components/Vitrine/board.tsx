// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the pages around Vitrine's receipt share, from the design boards Main,
// Revisit and Alert: the palette, the icons and the wrapper that brands the
// kit's elements with the portal's own colours.

import React from "react"
import {Box} from "@mui/material"
import {styled} from "@mui/material/styles"
import {theme} from "@sequentech/ui-essentials"

export const BOARD = {
    navy: theme.palette.brandColor,
    green: "#047857",
    greenTint: "#ECFDF5",
    greenLine: "#A7E3C6",
    warning: "#C2410C",
    warningText: "#9A3412",
    warningTint: "#FFF7ED",
    text: "#191D23",
    body: "#334155",
    muted: "#475569",
    subtle: "#64748B",
    border: "#E7EAEE",
    tint: "#F7F9FE",
    white: "#FFFFFF",
} as const

/** The kit's elements take their colours and corners from these custom
 * properties, so the pages around the receipt brand them with the portal's. */
export const VitrineBrand = styled(Box)`
    --vitrine-accent: ${BOARD.navy};
    --vitrine-text: ${BOARD.text};
    --vitrine-surface: ${BOARD.white};
    --vitrine-radius: 0px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    margin: 16px 0 24px;
`

export enum EGlyph {
    CHEVRON = "chevron",
    QR = "qr",
    SEARCH = "search",
    DOCUMENT = "document",
    CHECK = "check",
    EXCLAIM = "exclaim",
    GROWN = "grown",
    CONTRADICTION = "contradiction",
    REFRESH = "refresh",
    NETWORK = "network",
}

const PATHS: Record<EGlyph, string> = {
    [EGlyph.CHEVRON]: "<path d='M9 6l6 6-6 6'/>",
    [EGlyph.QR]:
        "<rect x='3.5' y='3.5' width='6.5' height='6.5'/><rect x='14' y='3.5' width='6.5' height='6.5'/><rect x='3.5' y='14' width='6.5' height='6.5'/><path d='M14 14h2.5v2.5H14zM18 18h2.5v2.5H18zM14 18.5v2M18.5 14h2'/>",
    [EGlyph.SEARCH]: "<circle cx='11' cy='11' r='6.5'/><path d='M16 16l4.5 4.5'/>",
    [EGlyph.DOCUMENT]: "<rect x='5' y='3.5' width='14' height='17'/><path d='M8.5 8h7M8.5 12h7'/>",
    [EGlyph.CHECK]: "<path d='M5 12.5l4.5 4.5L19 7.5'/>",
    [EGlyph.EXCLAIM]: "<path d='M12 5v9'/><path d='M12 19h.01'/>",
    [EGlyph.GROWN]: "<path d='M12 3v18M5 8l7-5 7 5'/>",
    [EGlyph.CONTRADICTION]: "<path d='M12 3v18M5 8l7-5 7 5'/><path d='M16 16l5 5M21 16l-5 5'/>",
    [EGlyph.REFRESH]: "<path d='M20 12a8 8 0 1 1-2.5-5.8'/><path d='M20 4v4.5h-4.5'/>",
    [EGlyph.NETWORK]:
        "<circle cx='12' cy='5.5' r='2.5'/><circle cx='5.5' cy='18' r='2.5'/><circle cx='18.5' cy='18' r='2.5'/><path d='M12 8v3.5M10.5 13.5L7 16M13.5 13.5L17 16'/>",
}

const maskOf = (glyph: EGlyph, width: number): string =>
    `url("data:image/svg+xml,${encodeURIComponent(
        `<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='${width}' stroke-linecap='round' stroke-linejoin='round'>${PATHS[glyph]}</svg>`
    )}")`

export interface IGlyphProps {
    glyph: EGlyph
    size?: number
    /** The stroke's width in the icon's 24-unit box. */
    stroke?: number
    className?: string
}

/** A decorative icon, drawn in the colour of the text around it. */
export const Glyph: React.FC<IGlyphProps> = ({glyph, size = 20, stroke = 1.8, className}) => (
    <Box
        component="span"
        aria-hidden="true"
        className={`vitrine-glyph ${className ?? ""}`.trim()}
        sx={{
            display: "inline-block",
            flex: "none",
            width: size,
            height: size,
            backgroundColor: "currentColor",
            maskImage: maskOf(glyph, stroke),
            maskSize: "contain",
            maskRepeat: "no-repeat",
            maskPosition: "center",
            WebkitMaskImage: maskOf(glyph, stroke),
            WebkitMaskSize: "contain",
            WebkitMaskRepeat: "no-repeat",
            WebkitMaskPosition: "center",
        }}
    />
)

export const headingSx = {
    margin: 0,
    fontSize: "24px",
    lineHeight: 1.2,
    fontWeight: 700,
    color: BOARD.navy,
} as const

export const quietTextSx = {fontSize: "14px", lineHeight: 1.45, color: BOARD.muted} as const

export const focusSx = {
    "&:focus-visible": {outline: `2px solid ${BOARD.navy}`, outlineOffset: "2px"},
} as const
