// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The selections an audited ballot decodes into, for the voter to compare
// with what they chose (design board AuditCompare; the voting portal's
// components/BallotLocator/TrackerAudit.tsx). They are drawn from memory and
// kept nowhere, but for the evidence file a voter chooses to save.

import React from "react"
import {useTranslation} from "react-i18next"
import Box from "@mui/material/Box"
import Typography from "@mui/material/Typography"
import Image from "mui-image"
import {BlankAnswer, Candidate} from "@sequentech/ui-essentials"
import {checkIsInvalidVote, getImageUrl, stringToHtml, translate} from "@sequentech/ui-core"
import type {IBallotStyle, ICandidate, IContest, IDecodedVoteContest} from "@sequentech/ui-core"
import {provideBallotService} from "../../services/BallotService"

const RANK_LAST = Number.MAX_SAFE_INTEGER

interface IChoice {
    candidate: ICandidate
    /** Where a ranked contest puts the candidate, from 1. */
    position?: number
    /** What the voter wrote in. */
    writeIn?: string
}

/** The candidates a decoded contest chose, in the contest's order, or in the
 * order of the voter's ranking. */
export const choicesOf = (
    contest: IContest,
    decoded: IDecodedVoteContest,
    ranked: boolean
): IChoice[] => {
    const chosen = contest.candidates.flatMap((candidate): IChoice[] => {
        if (checkIsInvalidVote(candidate)) return decoded.is_explicit_invalid ? [{candidate}] : []
        const choice = decoded.choices.find(({id}) => id === candidate.id)
        if (!choice || choice.selected < 0) return []
        return [
            {
                candidate,
                position: choice.selected + 1,
                ...(choice.write_in_text ? {writeIn: choice.write_in_text} : {}),
            },
        ]
    })
    return ranked
        ? [...chosen].sort((a, b) => (a.position ?? RANK_LAST) - (b.position ?? RANK_LAST))
        : chosen
}

interface IDecodedContestProps {
    contest: IContest
    decoded: IDecodedVoteContest
    language: string
    publicBucketUrl: string
}

const DecodedContest: React.FC<IDecodedContestProps> = ({
    contest,
    decoded,
    language,
    publicBucketUrl,
}) => {
    const {t} = useTranslation()
    const {isPreferential} = provideBallotService()
    const ranked = isPreferential(contest.counting_algorithm)
    const choices = choicesOf(contest, decoded, ranked)
    const headingId = `audited-selection-${contest.id}-title`

    return (
        <Box component="section" className="audited-selection" aria-labelledby={headingId}>
            <Typography
                component="h3"
                id={headingId}
                className="audited-selection-title"
                sx={{margin: "0 0 8px", fontSize: 14, lineHeight: "20px", fontWeight: 700}}
            >
                {translate(contest, "name", language)}
            </Typography>
            <Box
                component="ul"
                className="audited-selection-choices"
                sx={{
                    listStyle: "none",
                    margin: 0,
                    padding: 0,
                    display: "flex",
                    flexDirection: "column",
                    gap: "12px",
                }}
            >
                {decoded.is_decline_to_vote ? (
                    <BlankAnswer title={t("confirmationScreen.declineToVote")} />
                ) : choices.length === 0 ? (
                    <BlankAnswer />
                ) : (
                    choices.map(({candidate, position, writeIn}) => {
                        const imageUrl = getImageUrl(candidate)
                        return (
                            <Candidate
                                key={candidate.id}
                                title={translate(candidate, "name", language)}
                                description={
                                    writeIn ??
                                    stringToHtml(
                                        translate(candidate, "description", language) || ""
                                    )
                                }
                                isSelectable={false}
                                isInvalidVote={checkIsInvalidVote(candidate)}
                                isPreferentialVote={ranked}
                                selectedPosition={position}
                                totalCandidates={contest.candidates.length}
                                maxVotes={contest.max_votes}
                                checked={true}
                            >
                                {imageUrl ? (
                                    <Image
                                        className="candidate-image"
                                        src={`${publicBucketUrl}${imageUrl}`}
                                        duration={100}
                                    />
                                ) : null}
                            </Candidate>
                        )
                    })
                )}
            </Box>
        </Box>
    )
}

export interface IDecodedContestsProps {
    /** The ballot style that the audited ballot carries. */
    ballotStyle: IBallotStyle
    selections: IDecodedVoteContest[]
    language: string
    publicBucketUrl: string
}

/** The decoded contests, in the ballot style's order. */
export const DecodedContests: React.FC<IDecodedContestsProps> = ({
    ballotStyle,
    selections,
    language,
    publicBucketUrl,
}) => (
    <Box
        className="audited-selections-list"
        sx={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))",
            gap: "16px 20px",
        }}
    >
        {ballotStyle.contests.flatMap((contest) => {
            const decoded = selections.find(({contest_id}) => contest_id === contest.id)
            return decoded
                ? [
                      <DecodedContest
                          key={contest.id}
                          contest={contest}
                          decoded={decoded}
                          language={language}
                          publicBucketUrl={publicBucketUrl}
                      />,
                  ]
                : []
        })}
    </Box>
)
