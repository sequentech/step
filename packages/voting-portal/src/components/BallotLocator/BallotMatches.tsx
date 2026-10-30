// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The ballots that share a short ID, to choose from (design board
// TrackerSearch): each with its full ID and when it was cast.

import React from "react"
import {useTranslation} from "react-i18next"
import {Link} from "react-router-dom"
import {Box, Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import {shortBallotId, WIDE_GROUPS_PER_LINE} from "../../services/vitrine/ballotId"
import {BallotIdLines} from "./BallotIdLines"

const NAVY = "#0F054C"

const MatchRow = styled(Link)`
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 10px 14px;
    border: 1px solid rgba(0, 0, 0, 0.23);
    border-radius: 4px;
    color: ${NAVY};
    text-decoration: none;

    & + & {
        margin-top: 8px;
    }

    &:hover,
    &:focus-visible {
        border-color: ${NAVY};
        background: #f7f9fe;
    }

    @media (max-width: 750px) {
        flex-direction: column;
        align-items: flex-start;
        gap: 4px;
    }
`

export interface IBallotMatch {
    ballotId: string
    /** RFC 3339, when the ballot was cast. */
    createdAt?: string
}

export interface IBallotMatchesProps {
    matches: IBallotMatch[]
    /** The short ID the voter looked for. */
    lookedFor: string
    hrefOf: (ballotId: string) => string
    formatTime: (createdAt: string) => string
}

/** The list lives in a region that is always there, so that the voter hears
 * of the matches when they appear. */
export const BallotMatches: React.FC<IBallotMatchesProps> = ({
    matches,
    lookedFor,
    hrefOf,
    formatTime,
}) => {
    const {t} = useTranslation()
    return (
        <Box
            className="ballot-matches"
            role="status"
            sx={{marginTop: matches.length > 0 ? "16px" : 0, maxWidth: 680}}
        >
            {matches.length > 0 ? (
                <>
                    <Typography
                        className="ballot-matches-title"
                        sx={{margin: "0 0 8px", fontSize: 14, lineHeight: "20px"}}
                    >
                        {t("ballotLocator.matches.title", {
                            total: matches.length,
                            shortId: shortBallotId(lookedFor),
                        })}
                    </Typography>
                    {matches.map((match) => (
                        <MatchRow
                            key={match.ballotId}
                            className="ballot-match"
                            to={hrefOf(match.ballotId)}
                        >
                            <Box
                                className="ballot-match-id"
                                component="span"
                                sx={{
                                    font: "400 13px/19px ui-monospace,SFMono-Regular,Menlo,Consolas,monospace",
                                }}
                            >
                                <BallotIdLines
                                    ballotId={match.ballotId}
                                    perLine={WIDE_GROUPS_PER_LINE}
                                />
                            </Box>
                            {match.createdAt ? (
                                <Box
                                    className="ballot-match-time"
                                    component="span"
                                    sx={{flex: "none", fontSize: 13, color: "#64748B"}}
                                >
                                    {t("ballotLocator.matches.cast", {
                                        time: formatTime(match.createdAt),
                                    })}
                                </Box>
                            ) : null}
                        </MatchRow>
                    ))}
                </>
            ) : null}
        </Box>
    )
}
