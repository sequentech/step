// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Alert, Box, Typography} from "@mui/material"
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
    isAcclaimedContest,
    isEligibleAcclaimedCandidate,
    translateFromPresentation,
    stringToHtml,
    type ICategory,
} from "@sequentech/ui-core"
import Candidate from "../Candidate/Candidate"
import BlankAnswer from "../BlankAnswer/BlankAnswer"
import WarnBox, {EWarnBoxAnnouncement} from "../WarnBox/WarnBox"
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
    hasCategory?: boolean
    publicBucketUrl: string
    shouldDisable?: boolean
}

const CandidateChoice: React.FC<CandidateChoiceProps> = ({
    answer,
    choice,
    isWriteIn,
    isPreferentialVote,
    hasCategory,
    publicBucketUrl,
    shouldDisable,
}) => {
    const imageUrl = getImageUrl(answer)

    return (
        <Candidate
            title={answer.name || ""}
            description={answer.description}
            isWriteIn={isWriteIn && !shouldDisable}
            writeInValue={choice?.write_in_text}
            shouldDisable={shouldDisable}
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
    isAcclaimed: boolean
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
    isAcclaimed,
}) => {
    if (!isAcclaimed && !showCategoryOnReview(category, questionPlaintext)) {
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
        if (
            !isAcclaimed &&
            !shouldShowCategoryCandidateOnReview(category, candidate.id, choicesById)
        ) {
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
                shouldDisable={isAcclaimed}
            />
        )
    }

    return (
        <CandidatesList
            title={translate(listPresentation, "name", language) ?? categoryName}
            isActive={false}
            isCheckable={!!category.header}
            checked={isListSelected}
            shouldDisable={isAcclaimed}
        >
            {sortedSubtypes.map((subtypePresentation) => {
                const subtypeCandidates = sortedCandidates.filter(
                    (candidate) => subtypePresentation.name === candidate.presentation?.subtype
                )
                const subtypeCandidateIds = subtypeCandidates.map((candidate) => candidate.id)
                const hasSelectedAnswer = subtypeCandidateIds.some((candidateId) =>
                    isChoiceSelected(choicesById, candidateId)
                )

                if (
                    subtypeCandidates.length === 0 ||
                    (!isAcclaimed && !hasSelectedAnswer && !isListSelected)
                ) {
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
    acclamationDescription?: string
    defaultLanguageCode?: string
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
    acclamationDescription,
    defaultLanguageCode,
}) => {
    const {t, i18n} = useTranslation()

    if (!question) {
        return <>{contestNotFoundLabel}</>
    }

    const isPreferentialVote = isPreferential(question.counting_algorithm)
    const isAcclaimed = isAcclaimedContest(question)
    const choicesById = keyBy(questionPlaintext.choices, "id")
    const displayedQuestion = isAcclaimed
        ? {
              ...question,
              candidates: question.candidates.filter(isEligibleAcclaimedCandidate),
          }
        : question

    const explicitInvalidAnswer =
        (questionPlaintext.is_explicit_invalid &&
            question.presentation?.invalid_vote_policy !== EInvalidVotePolicy.NOT_ALLOWED &&
            question.candidates.find((answer) => checkIsInvalidVote(answer))) ||
        null
    const properties = getLayoutProperties(question)
    const isBlank = !isAcclaimed && checkIsBlank(questionPlaintext)

    const isBallotDeclineToVote =
        !isAcclaimed && isDeclineToVotePolicyEnabled && questionPlaintext.is_decline_to_vote

    const isWholeBallotBlank = Boolean(
        !isAcclaimed && isBlankBallotsPolicyEnabled && questionPlaintext.is_blank_ballot
    )

    const {noCategoryCandidates, categoriesMap} = categorizeCandidates(displayedQuestion)
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
    const displayedNoCategoryCandidates = isAcclaimed
        ? sortedNoCategoryCandidates
        : selectedNoCategoryCandidates
    const displayedAcclamationDescription = isAcclaimed
        ? translateFromPresentation(question, "acclamation_description", i18n.language, {
              defaultLanguageCode,
          }) || acclamationDescription
        : undefined

    return (
        <>
            <Typography variant="body2" fontWeight={"bold"}>
                {translate(question, "name", i18n.language) || ""}
            </Typography>
            {displayedAcclamationDescription ? (
                <Alert severity="info" className="contest-acclamation">
                    {stringToHtml(displayedAcclamationDescription)}
                </Alert>
            ) : null}
            {isWholeBallotBlank ? (
                <BlankAnswer title={blankBallotLabel} />
            ) : isBlank || isBallotDeclineToVote ? (
                <BlankAnswer title={isBallotDeclineToVote ? declineToVoteLabel : undefined} />
            ) : null}
            {!isBallotDeclineToVote && !isWholeBallotBlank && (
                <>
                    {!isAcclaimed &&
                        questionPlaintext.invalid_errors.map((error, index) => (
                            <WarnBox
                                variant="warning"
                                key={index}
                                // A decoded ballot is static: these boxes are rendered
                                // once with the contest and read in document order, so
                                // they must not turn into live regions in the portals
                                // that render them.
                                announcement={EWarnBoxAnnouncement.SILENT}
                                warnId={error.message}
                                warnType={error.error_type}
                            >
                                {t(error.message || "", normalizeMessageMap(error.message_map))}
                            </WarnBox>
                        ))}
                    {!isAcclaimed && questionPlaintext.is_explicit_invalid ? (
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
                                    question={displayedQuestion}
                                    questionPlaintext={questionPlaintext}
                                    choicesById={choicesById}
                                    isPreferentialVote={isPreferentialVote}
                                    publicBucketUrl={publicBucketUrl}
                                    language={i18n.language}
                                    isAcclaimed={isAcclaimed}
                                />
                            ))}
                        </CategoryListsWrapper>
                    ) : null}
                    {displayedNoCategoryCandidates.length > 0 ? (
                        <CandidatesWrapper>
                            {displayedNoCategoryCandidates.map((candidate) => (
                                <CandidateChoice
                                    key={candidate.id}
                                    answer={candidate}
                                    choice={choicesById[candidate.id]}
                                    isWriteIn={checkIsWriteIn(candidate)}
                                    isPreferentialVote={isPreferentialVote}
                                    publicBucketUrl={publicBucketUrl}
                                    shouldDisable={isAcclaimed}
                                />
                            ))}
                        </CandidatesWrapper>
                    ) : null}
                </>
            )}
        </>
    )
}
