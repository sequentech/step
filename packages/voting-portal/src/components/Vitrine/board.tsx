// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the booth's refusal dialog shares from the design board Alert: the
// palette, and the icons it draws.

import React from "react"
import {Box} from "@mui/material"

export const BOARD = {
    warning: "#C2410C",
    warningTint: "#FFF7ED",
    body: "#334155",
    muted: "#475569",
    subtle: "#64748B",
    tint: "#F7F9FE",
} as const

export enum EGlyph {
    EXCLAIM = "exclaim",
    REFRESH = "refresh",
}

const PATHS: Record<EGlyph, string> = {
    [EGlyph.EXCLAIM]: "<path d='M12 5v9'/><path d='M12 19h.01'/>",
    [EGlyph.REFRESH]: "<path d='M20 12a8 8 0 1 1-2.5-5.8'/><path d='M20 4v4.5h-4.5'/>",
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

export const quietTextSx = {fontSize: "14px", lineHeight: 1.45, color: BOARD.muted} as const
