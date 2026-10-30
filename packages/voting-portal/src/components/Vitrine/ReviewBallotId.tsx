// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The Ballot ID box of the review screen when the event keeps a transparency
// log (design board Review): Step's own box, with the short ID and, instead of
// the decorative check, the check mark of a ballot not cast yet.

import React from "react"
import {Box} from "@mui/material"
import {useTranslation} from "react-i18next"
import {faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import {BallotHashCopyButton, IconButton, theme} from "@sequentech/ui-essentials"
import {groupBallotId, shortBallotId} from "../../services/vitrine/ballotId"
import {EBallotStatus} from "../../services/vitrine/ballotStatus"
import {BallotStatus, EBallotStatusTrigger} from "./BallotStatus"
import {HEAVY_DOTS} from "./BallotStatusIcon"

const MONO = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"

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
    return (
        <Box
            className="hash-container vitrine-hash-container"
            sx={{
                display: "flex",
                alignItems: "center",
                gap: "8px",
                maxWidth: 700,
                mx: "auto",
                p: {xs: "6px 10px", sm: "10px 22px"},
                color: theme.palette.green.dark,
                border: `1px solid ${theme.palette.green.dark}`,
                borderRadius: "4px",
            }}
        >
            <BallotStatus
                trigger={EBallotStatusTrigger.ICON}
                status={EBallotStatus.NOT_CAST}
                ballotId={ballotId}
                electionName={electionName}
                strokeWidth={HEAVY_DOTS}
                sx={{m: "-7px -4px -7px -12px"}}
            />
            <Box
                className="hash-text"
                sx={{flex: 1, minWidth: 0, textAlign: "center", lineHeight: 1.5}}
            >
                {t("vitrine.ballotId.yours")}{" "}
                <Box
                    component="b"
                    className="vitrine-short-ballot-id"
                    sx={{fontFamily: MONO, fontWeight: 700, whiteSpace: "nowrap"}}
                >
                    {shortBallotId(ballotId)}
                </Box>
            </Box>
            <Box
                className="hash-actions"
                sx={{display: "flex", alignItems: "center", flexShrink: 0, ml: "8px"}}
            >
                <BallotHashCopyButton
                    hash={groupBallotId(ballotId)}
                    copyLabels={{
                        copy: t("vitrine.ballotId.copy"),
                        copied: t("vitrine.ballotId.copied"),
                        error: t("reviewScreen.ballotIdCopyError"),
                    }}
                />
                <IconButton
                    buttonClassName="hash-help-button"
                    icon={faCircleQuestion}
                    title={helpLabel}
                    sx={{
                        fontSize: "unset",
                        lineHeight: "unset",
                        paddingBottom: "2px",
                        color: theme.palette.customGrey.contrastText,
                    }}
                    fontSize="18px"
                    onClick={onHelpClick}
                    ariaLabel={helpLabel}
                />
            </Box>
        </Box>
    )
}
