// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The full Tracking ID in a block (design boards FullTrackingId and
// ReviewFixes), the same wherever it shows: as wide as the widget around it,
// its label top left and a labelled Copy button top right, always visible.
// The 13 groups are monospaced chips, the two of the short ID filled. They
// line up in one row, in 7 and 6, or in 5, 5 and 3, by the block's own width
// (container queries), so that no group is left alone. Where the browser
// refuses to copy, the chips show as selected for the voter to copy: no
// warning sign, as nothing went wrong with the ballot.

import React, {useId, useRef} from "react"
import {Box, ButtonBase} from "@mui/material"
import {visuallyHidden} from "@mui/utils"
import {useTranslation} from "react-i18next"
import {CopyBallotHashStatus} from "@sequentech/ui-essentials"
import {useCopyFeedback} from "../../hooks/useCopyFeedback"
import {ballotIdGroups, groupBallotId} from "../../services/vitrine/ballotId"

const MONO = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"
/** The groups of the short ID. */
const SHORT_GROUPS = 2

/** What the block sits on: it never shows as a plain white box. */
export enum EFullTrackingIdSurface {
    /** A page. */
    WHITE = "white",
    /** A light panel. */
    TINT = "tint",
    /** Inside the green Tracking ID card: the card's colour, behind a thin rule. */
    GREEN = "green",
    /** A hub's page, in the hub's own colour. */
    HUB = "hub",
}

const COLORS = {
    text: "#191D23",
    navy: "#0F054C",
    green: "#047857",
    greenDark: "#065F46",
    hub: "#0E4F55",
    slate: "#64748B",
    manual: "#334155",
    selected: "#C7D7FE",
    selectedLine: "#A5B8F7",
    chipLine: "#DCE1EE",
    buttonLine: "#CBD3E3",
    white: "#fff",
} as const

interface ISurfaceStyle {
    padding: string
    border: string
    borderTop: string
    borderRadius: string
    background: string
    boxShadow: string
    label: string
    key: string
    chipBackground: string
    chipLine: string
    buttonBackground: string
    buttonLine: string
    buttonColor: string
}

const surfaceStyle = (surface: EFullTrackingIdSurface): ISurfaceStyle => {
    if (surface === EFullTrackingIdSurface.GREEN) {
        return {
            padding: "12px 0 0",
            border: "0",
            borderTop: "1px solid rgba(4, 120, 87, 0.18)",
            borderRadius: "0",
            background: "transparent",
            boxShadow: "none",
            label: COLORS.green,
            key: COLORS.green,
            chipBackground: "rgba(255, 255, 255, 0.75)",
            chipLine: "rgba(4, 120, 87, 0.2)",
            buttonBackground: "rgba(255, 255, 255, 0.8)",
            buttonLine: "rgba(4, 120, 87, 0.3)",
            buttonColor: COLORS.greenDark,
        }
    }
    const tint = surface === EFullTrackingIdSurface.TINT
    const hub = surface === EFullTrackingIdSurface.HUB
    const line = tint ? "1px solid #DDE2F0" : "1px solid #E1E5F2"
    return {
        padding: "10px 10px 12px 12px",
        border: line,
        borderTop: line,
        borderRadius: "10px",
        background: tint
            ? "rgba(15, 5, 76, 0.035)"
            : "linear-gradient(180deg, #F8F9FE 0%, #F1F3FA 100%)",
        boxShadow: "inset 0 1px 0 rgba(255, 255, 255, 0.9)",
        label: COLORS.slate,
        key: hub ? COLORS.hub : COLORS.navy,
        chipBackground: COLORS.white,
        chipLine: COLORS.chipLine,
        buttonBackground: COLORS.white,
        buttonLine: COLORS.buttonLine,
        buttonColor: hub ? COLORS.hub : COLORS.navy,
    }
}

/** One row of 13 where it fits, else 7 and 6, else 5, 5 and 3; very narrow, the
 * chips get smaller. */
const CHIPS_SX = {
    "position": "relative",
    "display": "grid",
    "gridTemplateColumns": "repeat(7, max-content)",
    "gap": "6px",
    "@container (min-width: 720px)": {gridTemplateColumns: "repeat(13, max-content)"},
    "@container (max-width: 384px)": {gridTemplateColumns: "repeat(5, max-content)"},
    "@container (max-width: 268px)": {
        "gridTemplateColumns": "repeat(5, max-content)",
        "gap": "4px",
        "& .full-tracking-id-group": {
            padding: "2px 4px",
            fontSize: "12px",
            letterSpacing: "0.04em",
        },
    },
    "& ::selection": {backgroundColor: COLORS.selected, color: COLORS.navy},
} as const

const CopyIcon: React.FC = () => (
    <svg
        className="full-tracking-id-copy-icon"
        width="15"
        height="15"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2.2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
    >
        <rect
            className="full-tracking-id-copy-icon-box"
            x="8.5"
            y="8.5"
            width="11.5"
            height="11.5"
            rx="2"
        />
        <path
            className="full-tracking-id-copy-icon-path"
            d="M15.5 8.5V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v7.5a2 2 0 0 0 2 2h2.5"
        />
    </svg>
)

const CopiedIcon: React.FC = () => (
    <svg
        className="full-tracking-id-copied-icon"
        width="15"
        height="15"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="3"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
    >
        <path className="full-tracking-id-copied-mark" d="M4.5 12.5l4.75 4.75L19.5 7" />
    </svg>
)

export interface IFullTrackingIdProps {
    ballotId: string
    /** The id of the block, for a control that expands it. */
    id?: string
    className?: string
    surface?: EFullTrackingIdSurface
}

export const FullTrackingId: React.FC<IFullTrackingIdProps> = ({
    ballotId,
    id,
    className,
    surface = EFullTrackingIdSurface.WHITE,
}) => {
    const {t} = useTranslation()
    const {copied, manual, copy} = useCopyFeedback()
    const labelId = useId()
    const value = useRef<HTMLDivElement>(null)
    const groups = ballotIdGroups(ballotId)
    const look = surfaceStyle(surface)

    const onCopy = async () => {
        const result = await copy(groupBallotId(ballotId))
        const node = value.current
        const selection = window.getSelection()
        if (result !== CopyBallotHashStatus.Manual || !node || !selection) return
        const range = document.createRange()
        range.selectNodeContents(node)
        selection.removeAllRanges()
        selection.addRange(range)
    }

    const chip = (n: number) => {
        if (manual) {
            return {
                backgroundColor: COLORS.selected,
                borderColor: COLORS.selectedLine,
                color: COLORS.navy,
                fontWeight: n < SHORT_GROUPS ? 700 : 500,
            }
        }
        if (n < SHORT_GROUPS) {
            return {
                backgroundColor: look.key,
                borderColor: look.key,
                color: COLORS.white,
                fontWeight: 700,
            }
        }
        return {
            backgroundColor: look.chipBackground,
            borderColor: look.chipLine,
            color: COLORS.navy,
            fontWeight: 500,
        }
    }

    const button = copied
        ? {backgroundColor: COLORS.green, borderColor: COLORS.green, color: COLORS.white}
        : {
              backgroundColor: look.buttonBackground,
              borderColor: look.buttonLine,
              color: look.buttonColor,
          }

    return (
        <Box
            id={id}
            className={`full-tracking-id ${className ?? ""}`.trim()}
            data-surface={surface}
            sx={{
                boxSizing: "border-box",
                width: "100%",
                minWidth: 0,
                p: look.padding,
                border: look.border,
                borderTop: look.borderTop,
                borderRadius: look.borderRadius,
                background: look.background,
                boxShadow: look.boxShadow,
                color: COLORS.text,
                textAlign: "left",
            }}
        >
            <Box
                className="full-tracking-id-head"
                sx={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    gap: "8px",
                    minHeight: "32px",
                }}
            >
                <Box
                    id={labelId}
                    className="full-tracking-id-label"
                    sx={{
                        fontSize: "11px",
                        lineHeight: "14px",
                        fontWeight: 700,
                        letterSpacing: "0.07em",
                        textTransform: "uppercase",
                        color: look.label,
                    }}
                >
                    {t("vitrine.ballotId.full")}
                </Box>
                <ButtonBase
                    className="full-tracking-id-copy"
                    aria-label={t(copied ? "vitrine.ballotId.copied" : "vitrine.ballotId.copy")}
                    onClick={() => void onCopy()}
                    sx={{
                        "display": "inline-flex",
                        "alignItems": "center",
                        "gap": "6px",
                        "height": "32px",
                        "p": "0 12px 0 10px",
                        "border": "1px solid",
                        "borderRadius": "999px",
                        "flex": "none",
                        "fontFamily": "inherit",
                        "fontSize": "13px",
                        "lineHeight": 1,
                        "fontWeight": 600,
                        "transition": "background-color .15s, color .15s, border-color .15s",
                        ...button,
                        "&.Mui-focusVisible": {
                            outline: `2px solid ${COLORS.navy}`,
                            outlineOffset: "2px",
                            boxShadow: "none",
                        },
                    }}
                >
                    {copied ? <CopiedIcon /> : <CopyIcon />}
                    {t(copied ? "vitrine.ballotId.copiedText" : "vitrine.ballotId.copyText")}
                </ButtonBase>
            </Box>
            <Box
                className="full-tracking-id-chips-wrap"
                sx={{containerType: "inline-size", mt: "8px"}}
            >
                <Box
                    ref={value}
                    role="group"
                    aria-labelledby={labelId}
                    className="full-tracking-id-value"
                    sx={CHIPS_SX}
                >
                    {groups.map((group, n) => (
                        <Box
                            key={`${group}-${n}`}
                            component="span"
                            className={`full-tracking-id-group${n < SHORT_GROUPS ? " full-tracking-id-short" : ""}`}
                            sx={{
                                display: "inline-flex",
                                alignItems: "center",
                                justifyContent: "center",
                                p: "3px 6px",
                                border: "1px solid",
                                borderRadius: "6px",
                                fontFamily: MONO,
                                fontSize: "13px",
                                lineHeight: "20px",
                                letterSpacing: "0.06em",
                                whiteSpace: "nowrap",
                                fontVariantLigatures: "none",
                                ...chip(n),
                            }}
                        >
                            {group}
                            {n < groups.length - 1 ? (
                                <Box
                                    component="span"
                                    className="full-tracking-id-separator"
                                    sx={{
                                        position: "absolute",
                                        width: "1px",
                                        height: "1px",
                                        overflow: "hidden",
                                        clipPath: "inset(50%)",
                                        whiteSpace: "nowrap",
                                    }}
                                >
                                    -
                                </Box>
                            ) : null}
                        </Box>
                    ))}
                </Box>
            </Box>
            <Box
                className="full-tracking-id-status"
                role="status"
                sx={
                    manual
                        ? {mt: "8px", fontSize: "13px", lineHeight: "18px", color: COLORS.manual}
                        : visuallyHidden
                }
            >
                {manual
                    ? t("vitrine.ballotId.selected")
                    : copied
                      ? t("vitrine.ballotId.copied")
                      : ""}
            </Box>
        </Box>
    )
}
