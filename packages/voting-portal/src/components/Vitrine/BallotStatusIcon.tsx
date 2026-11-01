// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The check mark that shows a ballot's status (design board ReceiptDialog):
// one check, two, blue once everything is verified, with an amber clock when
// something is late and a red alert on a problem. An audited ballot has two
// grey checks, or one with the alert when the audit failed.

import React from "react"
import {keyframes, styled} from "@mui/material/styles"
import {EBallotStatus} from "../../services/vitrine/ballotStatus"

export enum ECheckMarks {
    SINGLE = "single",
    DOUBLE = "double",
}

export enum ECheckLine {
    SOLID = "solid",
    /** Not cast, or a step still to come. */
    DOTTED = "dotted",
    /** Being checked. */
    DASHED = "dashed",
}

export enum ECheckBadge {
    NONE = "none",
    CLOCK = "clock",
    ALERT = "alert",
}

export const CHECK_COLORS = {
    GREY: "#7D8799",
    LIGHT: "#CBD5E1",
    BLUE: "#1A5FD6",
    AMBER: "#B45309",
    RED: "#DC2626",
} as const

/** Sizes in pixels: next to a Tracking ID, in the dialog. */
export const CHECK_SIZES = {
    ID: 20,
    DIALOG: 28,
} as const

export interface ICheckGlyph {
    marks: ECheckMarks
    line: ECheckLine
    badge: ECheckBadge
    color: string
}

export const STATUS_GLYPHS: Record<EBallotStatus, ICheckGlyph> = {
    [EBallotStatus.NOT_CAST]: {
        marks: ECheckMarks.SINGLE,
        line: ECheckLine.DOTTED,
        badge: ECheckBadge.NONE,
        color: CHECK_COLORS.GREY,
    },
    [EBallotStatus.CHECKING]: {
        marks: ECheckMarks.SINGLE,
        line: ECheckLine.DASHED,
        badge: ECheckBadge.NONE,
        color: CHECK_COLORS.GREY,
    },
    [EBallotStatus.RECORDED]: {
        marks: ECheckMarks.SINGLE,
        line: ECheckLine.SOLID,
        badge: ECheckBadge.NONE,
        color: CHECK_COLORS.GREY,
    },
    [EBallotStatus.WITNESSED]: {
        marks: ECheckMarks.DOUBLE,
        line: ECheckLine.SOLID,
        badge: ECheckBadge.NONE,
        color: CHECK_COLORS.GREY,
    },
    [EBallotStatus.VERIFIED]: {
        marks: ECheckMarks.DOUBLE,
        line: ECheckLine.SOLID,
        badge: ECheckBadge.NONE,
        color: CHECK_COLORS.BLUE,
    },
    [EBallotStatus.LATE]: {
        marks: ECheckMarks.DOUBLE,
        line: ECheckLine.SOLID,
        badge: ECheckBadge.CLOCK,
        color: CHECK_COLORS.AMBER,
    },
    [EBallotStatus.PROBLEM]: {
        marks: ECheckMarks.SINGLE,
        line: ECheckLine.SOLID,
        badge: ECheckBadge.ALERT,
        color: CHECK_COLORS.RED,
    },
    [EBallotStatus.AUDITABLE]: {
        marks: ECheckMarks.DOUBLE,
        line: ECheckLine.SOLID,
        badge: ECheckBadge.NONE,
        color: CHECK_COLORS.GREY,
    },
    [EBallotStatus.AUDIT_FAILED]: {
        marks: ECheckMarks.SINGLE,
        line: ECheckLine.SOLID,
        badge: ECheckBadge.ALERT,
        color: CHECK_COLORS.RED,
    },
}

const SINGLE = "M4.5 12.5l4.75 4.75L19.5 7"
const DOUBLE = ["M1.5 12.75l4.5 4.5L16.25 7", "M10.75 15.75l1.5 1.5L22.5 7"]

const march = keyframes`
    to {
        stroke-dashoffset: -11;
    }
`

const MarchingPath = styled("path")`
    animation: ${march} 1s linear infinite;

    @media (prefers-reduced-motion: reduce) {
        animation: none;
    }
`

/** The stroke of the dotted check next to the Tracking ID on the review screen,
 * where thinner dots would read as a faint line. */
export const HEAVY_DOTS = 2.6

const dashOf = (line: ECheckLine, strokeWidth: number): string | undefined => {
    if (line === ECheckLine.DASHED) return "2.5 3"
    if (line === ECheckLine.DOTTED) return strokeWidth > 2 ? "0.01 3.3" : "0.01 3.2"
    return undefined
}

export interface ICheckIconProps {
    glyph: ICheckGlyph
    size?: number
    /** The background around the badge, so that it stands apart from the marks. */
    surface?: string
    strokeWidth?: number
    /** Whether the dashes of a check being made move. */
    animated?: boolean
}

export const CheckIcon: React.FC<ICheckIconProps> = ({
    glyph,
    size = CHECK_SIZES.ID,
    surface = "#fff",
    strokeWidth = 2,
    animated = true,
}) => {
    const dash = dashOf(glyph.line, strokeWidth)
    const paths = glyph.marks === ECheckMarks.SINGLE ? [SINGLE] : DOUBLE
    return (
        <svg
            className={`ballot-status-icon ballot-status-icon-${glyph.marks}`}
            width={size}
            height={size}
            viewBox="0 0 24 24"
            fill="none"
            stroke={glyph.color}
            strokeWidth={strokeWidth}
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
            focusable="false"
            style={{overflow: "visible", flex: "none"}}
        >
            {paths.map((d) =>
                glyph.line === ECheckLine.DASHED && animated ? (
                    <MarchingPath
                        key={d}
                        className="ballot-status-mark"
                        d={d}
                        strokeDasharray={dash}
                    />
                ) : (
                    <path key={d} className="ballot-status-mark" d={d} strokeDasharray={dash} />
                )
            )}
            {glyph.badge !== ECheckBadge.NONE && (
                <g className="ballot-status-badge">
                    <circle
                        className="ballot-status-badge-disc"
                        cx="19"
                        cy="18"
                        r="5"
                        fill={
                            glyph.badge === ECheckBadge.CLOCK
                                ? CHECK_COLORS.AMBER
                                : CHECK_COLORS.RED
                        }
                        stroke={surface}
                        strokeWidth="1.5"
                    />
                    {glyph.badge === ECheckBadge.CLOCK ? (
                        <path
                            className="ballot-status-badge-mark"
                            d="M19 15.7V18l1.6 1.1"
                            stroke="#fff"
                            strokeWidth="1.5"
                        />
                    ) : (
                        <>
                            <path
                                className="ballot-status-badge-mark"
                                d="M19 15.6v2.8"
                                stroke="#fff"
                                strokeWidth="1.6"
                            />
                            <path
                                className="ballot-status-badge-mark"
                                d="M19 20.5v.1"
                                stroke="#fff"
                                strokeWidth="1.7"
                            />
                        </>
                    )}
                </g>
            )}
        </svg>
    )
}

export interface IBallotStatusIconProps extends Omit<ICheckIconProps, "glyph"> {
    status: EBallotStatus
}

export const BallotStatusIcon: React.FC<IBallotStatusIconProps> = ({status, ...props}) => (
    <CheckIcon glyph={STATUS_GLYPHS[status]} {...props} />
)
