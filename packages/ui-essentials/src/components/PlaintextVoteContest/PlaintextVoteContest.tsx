// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import Image from "mui-image"
import {keyBy} from "lodash"
import {
    IDecodedVoteContest,
    IContest,
    ICandidate,
    EInvalidVotePolicy,
    translate,
    isPreferential,
    getLayoutProperties,
    getPoints,
    checkIsBlank,
    checkIsInvalidVote,
    checkIsWriteIn,
    getImageUrl,
} from "@sequentech/ui-core"
import Candidate from "../Candidate/Candidate"
import BlankAnswer from "../BlankAnswer/BlankAnswer"
import WarnBox from "../WarnBox/WarnBox"

const CandidatesWrapper = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin: 12px 0;
`

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

const normalizeMessageMap = (messageMap: unknown): Record<string, unknown> | undefined => {
    if (!messageMap) {
        return undefined
    }

    if (messageMap instanceof Map) {
        return Object.fromEntries(messageMap)
    }

    if (Array.isArray(messageMap)) {
        const isEntryTupleArray = messageMap.every(
            (entry): entry is [string, unknown] =>
                Array.isArray(entry) && entry.length === 2 && typeof entry[0] === "string"
        )
        return isEntryTupleArray ? Object.fromEntries(messageMap) : undefined
    }

    if (isRecord(messageMap)) {
        return messageMap
    }

    return undefined
}

interface VoteChoiceProps {
    text?: string
    points: number | null
    ordered: boolean
    pointsLabel: (points: number) => string
}

const VoteChoice: React.FC<VoteChoiceProps> = ({text, points, ordered, pointsLabel}) => {
    const content = (
        <Typography variant="body2">
            <li>
                <span>
                    {text} {points ? <>{pointsLabel(points)}</> : null}
                </span>
            </li>
        </Typography>
    )
    return ordered ? <ol>{content}</ol> : <ul>{content}</ul>
}

interface CandidateChoiceProps {
    answer?: ICandidate
    points: number | null
    isWriteIn: boolean
    writeInValue: string | undefined
    isPreferentialVote?: boolean
    selectedPosition?: number | null
    publicBucketUrl: string
    pointsLabel: (points: number) => string
}

const CandidateChoice: React.FC<CandidateChoiceProps> = ({
    answer,
    isWriteIn,
    writeInValue,
    isPreferentialVote,
    selectedPosition,
    publicBucketUrl,
}) => {
    const imageUrl = answer && getImageUrl(answer)

    return (
        <Candidate
            title={answer?.name || ""}
            description={answer?.description}
            isWriteIn={isWriteIn}
            writeInValue={writeInValue}
            shouldDisable={false}
            isPreferentialVote={isPreferentialVote}
            selectedPosition={selectedPosition}
        >
            {imageUrl ? <Image src={`${publicBucketUrl}${imageUrl}`} duration={100} /> : null}
        </Candidate>
    )
}

export interface PlaintextVoteContestProps {
    questionPlaintext: IDecodedVoteContest
    question: IContest | null
    publicBucketUrl: string
    contestNotFoundLabel: string
    markedInvalidLabel: string
    pointsLabel: (points: number) => string
    isDeclineToVotePolicyEnabled: boolean
    declineToVoteLabel?: string
}

export const PlaintextVoteContest: React.FC<PlaintextVoteContestProps> = ({
    questionPlaintext,
    question,
    publicBucketUrl,
    contestNotFoundLabel,
    markedInvalidLabel,
    pointsLabel,
    isDeclineToVotePolicyEnabled,
    declineToVoteLabel,
}) => {
    const {t, i18n} = useTranslation()

    if (!question) {
        return <>{contestNotFoundLabel}</>
    }

    const isPreferentialVote = isPreferential(question.counting_algorithm)
    const selectedAnswers = questionPlaintext.choices
        .filter((a) => a.selected > -1)
        .sort((a, b) => (isPreferentialVote ? a.selected - b.selected : 0))

    const explicitInvalidAnswer =
        (questionPlaintext.is_explicit_invalid &&
            question.presentation?.invalid_vote_policy !== EInvalidVotePolicy.NOT_ALLOWED &&
            question.candidates.find((answer) => checkIsInvalidVote(answer))) ||
        null
    const answersById = keyBy(question.candidates, (a) => a.id)
    const properties = getLayoutProperties(question)
    const showPoints = !!question.presentation?.show_points
    const isBlank = checkIsBlank(questionPlaintext)

    const isBallotDeclineToVote =
        isDeclineToVotePolicyEnabled && questionPlaintext.is_decline_to_vote

    return (
        <>
            <Typography variant="body2" fontWeight={"bold"}>
                {translate(question, "name", i18n.language) || ""}
            </Typography>
            {isBlank || isBallotDeclineToVote ? (
                <BlankAnswer title={isBallotDeclineToVote ? declineToVoteLabel : undefined} />
            ) : null}
            {!isBallotDeclineToVote && (
                <>
                    {questionPlaintext.invalid_errors.map((error, index) => (
                        <WarnBox variant="warning" key={index}>
                            {t(error.message || "", normalizeMessageMap(error.message_map))}
                        </WarnBox>
                    ))}
                    {questionPlaintext.is_explicit_invalid ? (
                        <VoteChoice
                            text={explicitInvalidAnswer?.name || markedInvalidLabel}
                            points={null}
                            ordered={properties?.ordered || false}
                            pointsLabel={pointsLabel}
                        />
                    ) : null}
                    <CandidatesWrapper>
                        {selectedAnswers.map((answer, index) => (
                            <CandidateChoice
                                key={index}
                                answer={answersById[answer.id]}
                                points={(showPoints && getPoints(question, answer)) || null}
                                isWriteIn={checkIsWriteIn(answersById[answer.id])}
                                writeInValue={answer.write_in_text}
                                isPreferentialVote={isPreferentialVote}
                                selectedPosition={answer.selected + 1}
                                publicBucketUrl={publicBucketUrl}
                                pointsLabel={pointsLabel}
                            />
                        ))}
                    </CandidatesWrapper>
                </>
            )}
        </>
    )
}
