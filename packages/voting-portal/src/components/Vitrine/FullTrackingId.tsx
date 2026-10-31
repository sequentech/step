// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The full Tracking ID in a framed block (design board FullTrackingId), the
// same wherever it shows: as wide as the widget around it, its label top left
// and its copy button top right, always visible. The ID is in groups of four
// that wrap only between groups, the short ID in bold. Where the browser
// refuses to copy, the ID is selected for the voter to copy: no warning sign,
// as nothing went wrong with the ballot.

import React, {useRef} from "react"
import {Box, ButtonBase} from "@mui/material"
import {visuallyHidden} from "@mui/utils"
import {useTranslation} from "react-i18next"
import {CopyBallotHashStatus} from "@sequentech/ui-essentials"
import {useCopyFeedback} from "../../hooks/useCopyFeedback"
import {ballotIdGroups, groupBallotId} from "../../services/vitrine/ballotId"

const MONO = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"
/** The groups of the short ID. */
const SHORT_GROUPS = 2

const COLORS = {
    border: "#E7EAEE",
    label: "#64748B",
    text: "#191D23",
    navy: "#0F054C",
    copied: "#047857",
} as const

const CopyIcon: React.FC = () => (
    <svg
        className="full-tracking-id-copy-icon"
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="currentColor"
        aria-hidden="true"
    >
        <path
            className="full-tracking-id-copy-front"
            d="M10 2h7.2L21 5.8V16a2 2 0 0 1-2 2h-9a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2z"
        />
        <path
            className="full-tracking-id-copy-back"
            d="M5 6h1v12a2 2 0 0 0 2 2h8v1a1 1 0 0 1-1 1H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2z"
        />
    </svg>
)

const CopiedIcon: React.FC = () => (
    <svg
        className="full-tracking-id-copied-icon"
        width="16"
        height="16"
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
    /** The id of the ID's text, for a control that expands the block. */
    id?: string
    className?: string
}

export const FullTrackingId: React.FC<IFullTrackingIdProps> = ({ballotId, id, className}) => {
    const {t} = useTranslation()
    const {copied, manual, copy} = useCopyFeedback()
    const value = useRef<HTMLDivElement>(null)
    const groups = ballotIdGroups(ballotId)

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

    return (
        <Box
            id={id}
            className={`full-tracking-id ${className ?? ""}`.trim()}
            sx={{
                boxSizing: "border-box",
                width: "100%",
                minWidth: 0,
                p: "8px 8px 10px 14px",
                border: `1px solid ${COLORS.border}`,
                borderRadius: "8px",
                bgcolor: "#fff",
                textAlign: "left",
            }}
        >
            <Box
                className="full-tracking-id-head"
                sx={{display: "flex", alignItems: "center", justifyContent: "space-between"}}
            >
                <Box
                    className="full-tracking-id-label"
                    sx={{fontSize: "12px", lineHeight: "16px", color: COLORS.label}}
                >
                    {t("vitrine.ballotId.full")}
                </Box>
                <ButtonBase
                    className="full-tracking-id-copy"
                    aria-label={t(copied ? "vitrine.ballotId.copied" : "vitrine.ballotId.copy")}
                    onClick={() => void onCopy()}
                    sx={{
                        "width": 34,
                        "height": 34,
                        "flex": "none",
                        "borderRadius": "50%",
                        "color": copied ? COLORS.copied : COLORS.text,
                        "&:hover": {bgcolor: "rgba(0, 0, 0, 0.04)"},
                        "&.Mui-focusVisible": {
                            outline: `2px solid ${COLORS.navy}`,
                            outlineOffset: "1px",
                            boxShadow: "none",
                        },
                    }}
                >
                    {copied ? <CopiedIcon /> : <CopyIcon />}
                </ButtonBase>
            </Box>
            <Box
                ref={value}
                className="full-tracking-id-value"
                sx={{
                    mt: "-4px",
                    pr: "6px",
                    fontFamily: MONO,
                    fontSize: "13px",
                    lineHeight: "20px",
                    color: COLORS.navy,
                }}
            >
                {groups.map((group, n) => (
                    <React.Fragment key={`${group}-${n}`}>
                        <Box
                            component="span"
                            className={`full-tracking-id-group${n < SHORT_GROUPS ? " full-tracking-id-short" : ""}`}
                            sx={{
                                whiteSpace: "nowrap",
                                fontWeight: n < SHORT_GROUPS ? 700 : 400,
                            }}
                        >
                            {n < groups.length - 1 ? `${group}-` : group}
                        </Box>
                        <wbr className="full-tracking-id-break" />
                    </React.Fragment>
                ))}
            </Box>
            <Box
                className="full-tracking-id-status"
                role="status"
                sx={
                    manual
                        ? {mt: "6px", fontSize: "12px", lineHeight: "16px", color: COLORS.label}
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
