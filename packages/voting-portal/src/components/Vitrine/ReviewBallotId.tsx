// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The Tracking ID box of the review screen when the event keeps a transparency
// log (design boards Review and PhoneReview): the check mark of a ballot not
// cast yet, the short ID with its tag, and the buttons to copy it and to ask
// about it. The short ID opens the full one below. It is one box: on a phone it
// takes the board's filled look, with its label above. The audit screen shows
// the same box for the ballot it audited (design board AuditScreen): a grey
// double check and the tag Audited.

import React, {useId, useState} from "react"
import {Box, ButtonBase} from "@mui/material"
import {useTranslation} from "react-i18next"
import {faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import {BallotHashCopyButton, IconButton, theme} from "@sequentech/ui-essentials"
import {groupBallotId, shortBallotId} from "../../services/vitrine/ballotId"
import {EBallotStatus, EBallotTag} from "../../services/vitrine/ballotStatus"
import type {IReceiptView} from "../../services/vitrine/castReceipt"
import {BallotStatus, EBallotStatusTrigger} from "./BallotStatus"
import {HEAVY_DOTS} from "./BallotStatusIcon"
import {BallotTag} from "./BallotTag"
import {EFullTrackingIdSurface, FullTrackingId} from "./FullTrackingId"
import {ID_BUTTON_SX} from "./idButtons"

const MONO = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"
const PHONE = theme.breakpoints.down("sm")

const COLORS = {
    green: theme.palette.green.dark,
    mint: theme.palette.green.light,
    text: theme.palette.customGrey.contrastText,
    navy: theme.palette.brandColor,
} as const

const focusSx = {
    "&.Mui-focusVisible": {outline: `2px solid ${COLORS.navy}`, outlineOffset: "1px"},
} as const

/** Which ballot the box shows. */
export enum EReviewBallotIdState {
    /** At review: encrypted, not cast yet. */
    NOT_CAST = "notCast",
    /** On the audit screen: audited, so never cast. */
    AUDITED = "audited",
}

export interface IReviewBallotIdProps {
    ballotId: string
    state?: EReviewBallotIdState
    /** The check of the ballot's `received` entry (at review) or its `audited`
     * one (audited), once the log answered. */
    view?: IReceiptView
    /** When the ballot was audited, as the voter reads it. */
    auditedAt?: string
    /** The audited ballot in the ballot tracker. */
    trackerUrl?: string
    electionName?: string
    helpLabel: string
    onHelpClick: () => void
    /** Offers the audit in the status dialog, when the event allows it. */
    onAudit?: () => void
}

export const ReviewBallotId: React.FC<IReviewBallotIdProps> = ({
    ballotId,
    state = EReviewBallotIdState.NOT_CAST,
    view,
    auditedAt,
    trackerUrl,
    electionName,
    helpLabel,
    onHelpClick,
    onAudit,
}) => {
    const {t} = useTranslation()
    const [expanded, setExpanded] = useState(false)
    const fullId = useId()
    const audited = state === EReviewBallotIdState.AUDITED
    const tag = audited ? EBallotTag.AUDITED : EBallotTag.NOT_CAST

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
                        {...(audited
                            ? {status: EBallotStatus.AUDITABLE, view, castAt: auditedAt, trackerUrl}
                            : view
                              ? {view}
                              : {status: EBallotStatus.NOT_CAST})}
                        tag={tag}
                        ballotId={ballotId}
                        electionName={electionName}
                        strokeWidth={audited ? undefined : HEAVY_DOTS}
                        onAudit={audited ? undefined : onAudit}
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
                        <BallotTag tag={tag} />
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
                        {expanded ? null : (
                            <BallotHashCopyButton
                                hash={groupBallotId(ballotId)}
                                copyLabels={{
                                    copy: t("vitrine.ballotId.copy"),
                                    copied: t("vitrine.ballotId.copied"),
                                    error: t("vitrine.ballotId.copyByHand"),
                                }}
                                copiedColor={COLORS.green}
                                sx={ID_BUTTON_SX}
                            />
                        )}
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
                    <FullTrackingId
                        id={fullId}
                        ballotId={ballotId}
                        surface={EFullTrackingIdSurface.GREEN}
                    />
                ) : null}
            </Box>
        </Box>
    )
}
