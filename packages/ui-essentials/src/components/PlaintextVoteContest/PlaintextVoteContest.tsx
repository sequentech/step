// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import Image from "mui-image"
import {keyBy, sortBy} from "lodash"
import {
    IDecodedVoteContest,
    IContest,
    ICandidate,
    EInvalidVotePolicy,
    translate,
    isPreferential,
    getLayoutProperties,
    checkIsBlank,
    checkIsInvalidVote,
    checkIsWriteIn,
    getImageUrl,
    sortCandidatesInContest,
    IDecodedVoteChoice,
    categorizeCandidates,
    sortCategoryEntries,
    showCategoryOnReview,
    isChoiceSelected,
    isCategoryListSelected,
    shouldShowCategoryCandidateOnReview,
    type ICategory,
} from "@sequentech/ui-core"
import Candidate from "../Candidate/Candidate"
import BlankAnswer from "../BlankAnswer/BlankAnswer"
import WarnBox from "../WarnBox/WarnBox"
import CandidatesList from "../CandidatesList/CandidatesList"

const CandidatesWrapper = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin: 12px 0;
`

const CategoryListsWrapper = styled(Box)`
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
    answer: ICandidate
    choice?: IDecodedVoteChoice
    isWriteIn: boolean
    isPreferentialVote?: boolean
    publicBucketUrl: string
    hasCategory?: boolean
}

const CandidateChoice: React.FC<CandidateChoiceProps> = ({
    answer,
    choice,
    isWriteIn,
    isPreferentialVote,
    hasCategory,
    publicBucketUrl,
}) => {
    const imageUrl = getImageUrl(answer)

    return (
        <Candidate
            title={answer.name || ""}
            description={answer.description}
            isWriteIn={isWriteIn}
            writeInValue={choice?.write_in_text}
            shouldDisable={false}
            isSelectable={false}
            hasCategory={hasCategory}
            isPreferentialVote={isPreferentialVote}
            selectedPosition={choice?.selected ? choice.selected + 1 : null}
        >
            {imageUrl ? <Image src={`${publicBucketUrl}${imageUrl}`} duration={100} /> : null}
        </Candidate>
    )
}

interface CategoryVoteListProps {
    categoryName: string
    category: ICategory
    question: IContest
    questionPlaintext: IDecodedVoteContest
    choicesById: Record<string, IDecodedVoteChoice>
    isPreferentialVote: boolean
    publicBucketUrl: string
    language: string
}

const CategoryVoteList: React.FC<CategoryVoteListProps> = ({
    categoryName,
    category,
    question,
    questionPlaintext,
    choicesById,
    isPreferentialVote,
    publicBucketUrl,
    language,
}) => {
    if (!showCategoryOnReview(category, questionPlaintext)) {
        return null
    }

    const isListSelected = isCategoryListSelected(category, choicesById)
    const candidatesOrderType = question.presentation?.candidates_order
    const sortedCandidates = sortCandidatesInContest(category.candidates, candidatesOrderType, true)

    let listPresentation = question.presentation?.types_presentation?.[categoryName] ?? {
        name: categoryName,
    }
    listPresentation = {...listPresentation, name: categoryName}

    const subtypesPresentation = Object.entries(listPresentation.subtypes_presentation ?? {}).map(
        ([key, value]) => ({
            ...value,
            name: key,
            sort_order: value.sort_order ?? 0,
        })
    )
    const sortedSubtypes = sortBy(subtypesPresentation, ["sort_order"])

    const renderCandidate = (candidate: ICandidate, hasCategory = true) => {
        if (!shouldShowCategoryCandidateOnReview(category, candidate.id, choicesById)) {
            return null
        }

        return (
            <CandidateChoice
                key={candidate.id}
                answer={candidate}
                choice={choicesById[candidate.id]}
                isWriteIn={checkIsWriteIn(candidate)}
                isPreferentialVote={isPreferentialVote}
                hasCategory={hasCategory}
                publicBucketUrl={publicBucketUrl}
            />
        )
    }

    return (
        <CandidatesList
            title={translate(listPresentation, "name", language) ?? categoryName}
            isActive={false}
            isCheckable={!!category.header}
            checked={isListSelected}
        >
            {sortedSubtypes.map((subtypePresentation) => {
                const subtypeCandidates = sortedCandidates.filter(
                    (candidate) => subtypePresentation.name === candidate.presentation?.subtype
                )
                const subtypeCandidateIds = subtypeCandidates.map((candidate) => candidate.id)
                const hasSelectedAnswer = subtypeCandidateIds.some((candidateId) =>
                    isChoiceSelected(choicesById, candidateId)
                )

                if (subtypeCandidates.length === 0 || (!hasSelectedAnswer && !isListSelected)) {
                    return null
                }

                return (
                    <React.Fragment key={subtypePresentation.name}>
                        <b>{translate(subtypePresentation, "name", language)}</b>
                        {subtypeCandidates.map((candidate) => renderCandidate(candidate))}
                    </React.Fragment>
                )
            })}
            {sortedCandidates
                .filter((candidate) => !candidate.presentation?.subtype)
                .map((candidate) => renderCandidate(candidate))}
        </CandidatesList>
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
    isBlankBallotsPolicyEnabled?: boolean
    blankBallotLabel?: string
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
    isBlankBallotsPolicyEnabled,
    blankBallotLabel,
}) => {
    const {t, i18n} = useTranslation()

    if (!question) {
        return <>{contestNotFoundLabel}</>
    }

    const isPreferentialVote = isPreferential(question.counting_algorithm)
    const choicesById = keyBy(questionPlaintext.choices, "id")

    const explicitInvalidAnswer =
        (questionPlaintext.is_explicit_invalid &&
            question.presentation?.invalid_vote_policy !== EInvalidVotePolicy.NOT_ALLOWED &&
            question.candidates.find((answer) => checkIsInvalidVote(answer))) ||
        null
    const properties = getLayoutProperties(question)
    const isBlank = checkIsBlank(questionPlaintext)

    const isBallotDeclineToVote =
        isDeclineToVotePolicyEnabled && questionPlaintext.is_decline_to_vote

    const isWholeBallotBlank = Boolean(
        isBlankBallotsPolicyEnabled && questionPlaintext.is_blank_ballot
    )

    const {noCategoryCandidates, categoriesMap} = categorizeCandidates(question)
    const sortedCategoryEntries = sortCategoryEntries(
        categoriesMap,
        question.presentation?.types_presentation
    )
    const candidatesOrderType = question.presentation?.candidates_order
    const sortedNoCategoryCandidates = sortCandidatesInContest(
        noCategoryCandidates,
        candidatesOrderType,
        true
    )
    const selectedNoCategoryCandidates = sortedNoCategoryCandidates
        .filter((candidate) => isChoiceSelected(choicesById, candidate.id))
        .sort((a, b) => {
            if (!isPreferentialVote) {
                return 0
            }
            return (choicesById[a.id]?.selected ?? -1) - (choicesById[b.id]?.selected ?? -1)
        })

    return (
        <>
            <Typography variant="body2" fontWeight={"bold"}>
                {translate(question, "name", i18n.language) || ""}
            </Typography>
            {isWholeBallotBlank ? (
                <BlankAnswer title={blankBallotLabel} />
            ) : isBlank || isBallotDeclineToVote ? (
                <BlankAnswer title={isBallotDeclineToVote ? declineToVoteLabel : undefined} />
            ) : null}
            {!isBallotDeclineToVote && !isWholeBallotBlank && (
                <>
                    {questionPlaintext.invalid_errors.map((error, index) => (
                        <WarnBox
                            variant="warning"
                            key={index}
                            warnId={error.message}
                            warnType={error.error_type}
                        >
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
                    {sortedCategoryEntries.length > 0 ? (
                        <CategoryListsWrapper>
                            {sortedCategoryEntries.map(([categoryName, category]) => (
                                <CategoryVoteList
                                    key={categoryName}
                                    categoryName={categoryName}
                                    category={category}
                                    question={question}
                                    questionPlaintext={questionPlaintext}
                                    choicesById={choicesById}
                                    isPreferentialVote={isPreferentialVote}
                                    publicBucketUrl={publicBucketUrl}
                                    language={i18n.language}
                                />
                            ))}
                        </CategoryListsWrapper>
                    ) : null}
                    {selectedNoCategoryCandidates.length > 0 ? (
                        <CandidatesWrapper>
                            {selectedNoCategoryCandidates.map((candidate) => (
                                <CandidateChoice
                                    key={candidate.id}
                                    answer={candidate}
                                    choice={choicesById[candidate.id]}
                                    isWriteIn={checkIsWriteIn(candidate)}
                                    isPreferentialVote={isPreferentialVote}
                                    publicBucketUrl={publicBucketUrl}
                                />
                            ))}
                        </CandidatesWrapper>
                    ) : null}
                </>
            )}
        </>
    )
}
