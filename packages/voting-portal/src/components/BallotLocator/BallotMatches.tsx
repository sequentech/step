// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The ballots that share a short ID, to choose from (design board
// TrackerSearch): each with its full ID, and when it was cast or audited.

import React from "react"
import {useTranslation} from "react-i18next"
import {Link} from "react-router-dom"
import {Box, Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import {translateHtml} from "@sequentech/ui-core"
import {shortBallotId, WIDE_GROUPS_PER_LINE} from "../../services/vitrine/ballotId"
import {ERecordKind} from "../../services/vitrine/ballotTracker"
import {BallotIdLines} from "./BallotIdLines"
import {MONO, PALETTE, PHONE} from "./trackerStyle"

/** How the list looks: plain rows in the locator of events without a public
 * record, cards in the ballot tracker. */
export enum EMatchesLook {
    ROWS = "rows",
    CARDS = "cards",
}

const MatchRow = styled(Link, {shouldForwardProp: (prop) => prop !== "look"})<{
    look: EMatchesLook
}>(({look}) => {
    const cards = look === EMatchesLook.CARDS
    return {
        "display": "flex",
        "alignItems": "center",
        "justifyContent": "space-between",
        "gap": 16,
        "padding": cards ? "14px 16px" : "10px 14px",
        "border": `1px solid ${cards ? PALETTE.border : "rgba(0, 0, 0, 0.23)"}`,
        "borderRadius": cards ? 12 : 4,
        "backgroundColor": cards ? "#fff" : undefined,
        "color": PALETTE.navy,
        "textDecoration": "none",
        "transition": cards ? "border-color .15s, box-shadow .15s" : undefined,
        "& + &": {marginTop: 8},
        "&:hover, &:focus-visible": cards
            ? {borderColor: PALETTE.navy, boxShadow: "0 2px 10px rgba(15, 5, 76, 0.08)"}
            : {borderColor: PALETTE.navy, background: "#f7f9fe"},
        [PHONE]: {flexDirection: "column", alignItems: "flex-start", gap: 4},
    }
})

export interface IBallotMatch {
    ballotId: string
    /** What the ballot's record holds, which says whether it was cast. */
    kind: ERecordKind
    /** RFC 3339, when the ballot was cast. */
    createdAt?: string
}

export interface IBallotMatchesProps {
    matches: IBallotMatch[]
    /** The short ID the voter looked for. */
    lookedFor: string
    hrefOf: (ballotId: string) => string
    formatTime: (createdAt: string) => string
    look?: EMatchesLook
}

/** The list lives in a region that is always there, so that the voter hears
 * of the matches when they appear. */
export const BallotMatches: React.FC<IBallotMatchesProps> = ({
    matches,
    lookedFor,
    hrefOf,
    formatTime,
    look = EMatchesLook.ROWS,
}) => {
    const {t} = useTranslation()
    const cards = look === EMatchesLook.CARDS
    return (
        <Box
            className="ballot-matches"
            role="status"
            sx={{
                marginTop: matches.length > 0 ? (cards ? "24px" : "16px") : 0,
                width: cards ? "100%" : undefined,
                maxWidth: cards ? undefined : 680,
                textAlign: "left",
            }}
        >
            {matches.length > 0 ? (
                <>
                    <Typography
                        className="ballot-matches-title"
                        component="p"
                        sx={{
                            margin: cards ? "0 0 10px 4px" : "0 0 8px",
                            fontSize: 14,
                            lineHeight: "20px",
                        }}
                    >
                        {translateHtml(t, "ballotLocator.matches.title", {
                            total: matches.length,
                            shortId: shortBallotId(lookedFor),
                        })}
                    </Typography>
                    {matches.map((match) => (
                        <MatchRow
                            key={match.ballotId}
                            className="ballot-match"
                            to={hrefOf(match.ballotId)}
                            look={look}
                        >
                            <Box
                                className="ballot-match-id"
                                component="span"
                                sx={{font: `400 13px/19px ${MONO}`}}
                            >
                                <BallotIdLines
                                    ballotId={match.ballotId}
                                    perLine={WIDE_GROUPS_PER_LINE}
                                />
                            </Box>
                            {match.createdAt || cards ? (
                                <Box
                                    className="ballot-match-time"
                                    component="span"
                                    sx={{
                                        flex: "none",
                                        display: "flex",
                                        alignItems: "center",
                                        gap: "6px",
                                        fontSize: 13,
                                        color: PALETTE.quiet,
                                    }}
                                >
                                    {match.createdAt
                                        ? t(
                                              match.kind === ERecordKind.AUDITED
                                                  ? "ballotLocator.matches.audited"
                                                  : "ballotLocator.matches.cast",
                                              {time: formatTime(match.createdAt)}
                                          )
                                        : null}
                                    {cards ? (
                                        <svg
                                            className="ballot-match-arrow"
                                            width="16"
                                            height="16"
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            strokeWidth="2.4"
                                            strokeLinecap="round"
                                            strokeLinejoin="round"
                                            aria-hidden="true"
                                            focusable="false"
                                        >
                                            <path
                                                className="ballot-match-arrow-path"
                                                d="M9 5l7 7-7 7"
                                            />
                                        </svg>
                                    ) : null}
                                </Box>
                            ) : null}
                        </MatchRow>
                    ))}
                </>
            ) : null}
        </Box>
    )
}
