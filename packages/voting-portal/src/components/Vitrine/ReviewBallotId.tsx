// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The Tracking ID box of the review screen when the event keeps a transparency
// log (design boards Review and PhoneReview): the check mark of a ballot not
// cast yet, the short ID with its tag, and the buttons to copy it and to ask
// about it. The short ID opens the full one below. It is one box: on a phone it
// takes the board's filled look, with its label above.

import React, {useId, useState} from "react"
import {Box, ButtonBase} from "@mui/material"
import {useTranslation} from "react-i18next"
import {faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import {BallotHashCopyButton, IconButton, theme} from "@sequentech/ui-essentials"
import {ballotIdGroups, groupBallotId, shortBallotId} from "../../services/vitrine/ballotId"
import {EBallotStatus, EBallotTag} from "../../services/vitrine/ballotStatus"
import {BallotStatus, EBallotStatusTrigger} from "./BallotStatus"
import {HEAVY_DOTS} from "./BallotStatusIcon"
import {BallotTag} from "./BallotTag"
import {ID_BUTTON_SX} from "./idButtons"

const MONO = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"
const PHONE = theme.breakpoints.down("sm")

const COLORS = {
    green: theme.palette.green.dark,
    mint: theme.palette.green.light,
    text: theme.palette.customGrey.contrastText,
    label: theme.palette.customGrey.dark,
    navy: theme.palette.brandColor,
} as const

const focusSx = {
    "&.Mui-focusVisible": {outline: `2px solid ${COLORS.navy}`, outlineOffset: "1px"},
} as const

/** The full ID, with its first two groups apart from the rest. */
const fullIdParts = (ballotId: string): {lead: string; rest: string} => {
    const groups = ballotIdGroups(ballotId)
    return {
        lead: groups.slice(0, 2).join(" "),
        rest: groups.length > 2 ? ` ${groups.slice(2).join(" ")}` : "",
    }
}

export interface IReviewBallotIdProps {
    ballotId: string
    electionName?: string
    helpLabel: string
    onHelpClick: () => void
}

export const ReviewBallotId: React.FC<IReviewBallotIdProps> = ({
    ballotId,
    electionName,
    helpLabel,
    onHelpClick,
}) => {
    const {t} = useTranslation()
    const [expanded, setExpanded] = useState(false)
    const fullId = useId()
    const full = fullIdParts(ballotId)

    return (
        <Box className="vitrine-tracking-id">
            <Box
                component="h2"
                className="vitrine-tracking-id-label"
                sx={{
                    display: "none",
                    [PHONE]: {
                        display: "block",
                        m: "0 0 8px",
                        fontSize: "18px",
                        lineHeight: 1.334,
                        fontWeight: 700,
                    },
                }}
            >
                {t("vitrine.ballotId.label")}
            </Box>
            <Box
                className="hash-container vitrine-hash-container"
                sx={{
                    boxSizing: "border-box",
                    display: "flex",
                    flexDirection: "column",
                    gap: "6px",
                    p: "6px 14px",
                    color: COLORS.green,
                    border: `1px solid ${COLORS.green}`,
                    borderRadius: "8px",
                    [PHONE]: {
                        width: "100%",
                        p: "10px 12px",
                        color: COLORS.text,
                        bgcolor: COLORS.mint,
                        border: 0,
                    },
                }}
            >
                <Box
                    className="vitrine-tracking-id-row"
                    sx={{
                        display: "flex",
                        alignItems: "center",
                        gap: "8px",
                        minWidth: 0,
                        [PHONE]: {gap: "10px"},
                    }}
                >
                    <BallotStatus
                        trigger={EBallotStatusTrigger.ICON}
                        status={EBallotStatus.NOT_CAST}
                        ballotId={ballotId}
                        electionName={electionName}
                        strokeWidth={HEAVY_DOTS}
                        sx={{m: "-7px -4px -7px -12px", [PHONE]: {m: "-3px -3px -3px -5px"}}}
                    />
                    <Box
                        className="hash-text vitrine-tracking-id-text"
                        sx={{
                            flex: 1,
                            minWidth: 0,
                            display: "flex",
                            flexWrap: "wrap",
                            alignItems: "center",
                            justifyContent: "center",
                            gap: "4px 10px",
                            fontSize: "16px",
                            lineHeight: 1.5,
                            [PHONE]: {justifyContent: "flex-start", lineHeight: "22px"},
                        }}
                    >
                        <Box component="span" className="vitrine-tracking-id-short">
                            <Box
                                component="span"
                                className="vitrine-tracking-id-prefix"
                                sx={{[PHONE]: {display: "none"}}}
                            >
                                {t("vitrine.ballotId.yours")}{" "}
                            </Box>
                            <ButtonBase
                                className="vitrine-short-ballot-id"
                                aria-expanded={expanded}
                                aria-controls={expanded ? fullId : undefined}
                                title={t("vitrine.ballotId.showFull")}
                                onClick={() => setExpanded((open) => !open)}
                                sx={[
                                    {
                                        "font": "inherit",
                                        "fontFamily": MONO,
                                        "fontWeight": 700,
                                        "whiteSpace": "nowrap",
                                        "verticalAlign": "baseline",
                                        "&:hover": {textDecoration: "underline"},
                                        [PHONE]: {color: COLORS.navy, lineHeight: "22px"},
                                    },
                                    focusSx,
                                ]}
                            >
                                {shortBallotId(ballotId)}
                            </ButtonBase>
                        </Box>
                        <BallotTag tag={EBallotTag.NOT_CAST} />
                    </Box>
                    <Box
                        className="hash-actions"
                        sx={{
                            display: "flex",
                            alignItems: "center",
                            flexShrink: 0,
                            ml: "8px",
                            [PHONE]: {gap: "10px", ml: 0},
                        }}
                    >
                        <BallotHashCopyButton
                            hash={groupBallotId(ballotId)}
                            copyLabels={{
                                copy: t("vitrine.ballotId.copy"),
                                copied: t("vitrine.ballotId.copied"),
                                error: t("reviewScreen.ballotIdCopyError"),
                            }}
                            copiedColor={COLORS.green}
                            sx={ID_BUTTON_SX}
                        />
                        <IconButton
                            buttonClassName="hash-help-button"
                            icon={faCircleQuestion}
                            title={helpLabel}
                            sx={{
                                fontSize: "unset",
                                lineHeight: "unset",
                                color: COLORS.text,
                                ...ID_BUTTON_SX,
                            }}
                            fontSize="18px"
                            onClick={onHelpClick}
                            ariaLabel={helpLabel}
                        />
                    </Box>
                </Box>
                {expanded ? (
                    <Box
                        id={fullId}
                        className="vitrine-tracking-id-full"
                        sx={{
                            // Wraps inside the box instead of widening it.
                            width: 0,
                            minWidth: "100%",
                            boxSizing: "border-box",
                            pl: "26px",
                            [PHONE]: {pl: "38px"},
                        }}
                    >
                        <Box
                            className="vitrine-tracking-id-full-label"
                            sx={{fontSize: "12px", lineHeight: "16px", color: COLORS.label}}
                        >
                            {t("vitrine.ballotId.full")}
                        </Box>
                        <Box
                            className="vitrine-tracking-id-full-value"
                            sx={{
                                fontFamily: MONO,
                                fontSize: "13px",
                                lineHeight: "20px",
                                color: COLORS.navy,
                                overflowWrap: "anywhere",
                            }}
                        >
                            <Box
                                component="b"
                                className="vitrine-tracking-id-full-lead"
                                sx={{fontWeight: 700}}
                            >
                                {full.lead}
                            </Box>
                            {full.rest}
                        </Box>
                    </Box>
                ) : null}
            </Box>
        </Box>
    )
}
