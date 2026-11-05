// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The check mark that shows a ballot's status (design board ReceiptDialog):
// one check, two, blue once everything is verified, with an amber clock when
// something is late and a red alert on a problem.

import React from "react"
import {keyframes, styled} from "@mui/material/styles"
import {EBallotStatus, EStatusStep, EStepState} from "../../services/vitrine/ballotStatus"

export enum ECheckMarks {
    SINGLE = "single",
    DOUBLE = "double",
}

export enum ECheckLine {
    SOLID = "solid",
    /** Being checked. */
    DASHED = "dashed",
    /** A step that waits for the one before it. */
    QUEUED = "queued",
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

/** Sizes in pixels: in a heading, in a step's row, in the status. */
export const CHECK_SIZES = {
    HEADING: 22,
    ROW: 20,
    STATUS: 28,
} as const

export interface ICheckGlyph {
    marks: ECheckMarks
    line: ECheckLine
    badge: ECheckBadge
    color: string
}

export const STATUS_GLYPHS: Record<EBallotStatus, ICheckGlyph> = {
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
}

/** The check of a step's row: one check for the record, two for the steps
 * that follow, and blue only for the anchors, the last one. */
export const stepGlyph = (step: EStatusStep, state: EStepState): ICheckGlyph => {
    const marks = step === EStatusStep.RECORD ? ECheckMarks.SINGLE : ECheckMarks.DOUBLE
    const glyph = (line: ECheckLine, badge: ECheckBadge, color: string): ICheckGlyph => ({
        marks,
        line,
        badge,
        color,
    })
    switch (state) {
        case EStepState.DONE:
            return glyph(
                ECheckLine.SOLID,
                ECheckBadge.NONE,
                step === EStatusStep.ANCHORS ? CHECK_COLORS.BLUE : CHECK_COLORS.GREY
            )
        case EStepState.ACTIVE:
            return glyph(ECheckLine.DASHED, ECheckBadge.NONE, CHECK_COLORS.GREY)
        case EStepState.LATE:
            return glyph(ECheckLine.SOLID, ECheckBadge.CLOCK, CHECK_COLORS.AMBER)
        case EStepState.FAILED:
        case EStepState.DAMAGED:
            return glyph(ECheckLine.SOLID, ECheckBadge.ALERT, CHECK_COLORS.RED)
        default:
            return glyph(ECheckLine.QUEUED, ECheckBadge.NONE, CHECK_COLORS.LIGHT)
    }
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

const DASHES = "2.5 3"

export interface ICheckIconProps {
    glyph: ICheckGlyph
    size?: number
    /** The background around the badge, so that it stands apart from the marks. */
    surface?: string
}

export const CheckIcon: React.FC<ICheckIconProps> = ({
    glyph,
    size = CHECK_SIZES.ROW,
    surface = "#fff",
}) => {
    const paths = glyph.marks === ECheckMarks.SINGLE ? [SINGLE] : DOUBLE
    const dash = glyph.line === ECheckLine.SOLID ? undefined : DASHES
    return (
        <svg
            width={size}
            height={size}
            viewBox="0 0 24 24"
            fill="none"
            stroke={glyph.color}
            strokeWidth={2}
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
            focusable="false"
            style={{overflow: "visible", flex: "none"}}
        >
            {paths.map((d) =>
                glyph.line === ECheckLine.DASHED ? (
                    <MarchingPath key={d} d={d} strokeDasharray={dash} />
                ) : (
                    <path key={d} d={d} strokeDasharray={dash} />
                )
            )}
            {glyph.badge !== ECheckBadge.NONE && (
                <g>
                    <circle
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
                        <path d="M19 15.7V18l1.6 1.1" stroke="#fff" strokeWidth="1.5" />
                    ) : (
                        <>
                            <path d="M19 15.6v2.8" stroke="#fff" strokeWidth="1.6" />
                            <path d="M19 20.5v.1" stroke="#fff" strokeWidth="1.7" />
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
