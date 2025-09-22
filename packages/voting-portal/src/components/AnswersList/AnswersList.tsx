// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {CandidatesList} from "@sequentech/ui-essentials"
import {IDecodedVoteContest, isUndefined, IContest, translate, keyBy} from "@sequentech/ui-core"
import {Answer} from "../Answer/Answer"
import {useAppDispatch, useAppSelector} from "../../store/hooks"
import {
    resetBallotSelection,
    selectBallotSelectionQuestion,
    selectBallotSelectionVoteChoice,
    setBallotSelectionVoteChoice,
} from "../../store/ballotSelections/ballotSelectionsSlice"
import {ICategory} from "../../services/CategoryService"
import {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {useTranslation} from "react-i18next"
import {sortBy} from "lodash"
import {sortCandidatesInContest, ECandidatesIconCheckboxPolicy} from "@sequentech/ui-core"

export interface AnswersListProps {
    title: string
    isActive: boolean
    checkableLists: boolean
    checkableCandidates: boolean
    iconCheckboxPolicy?: ECandidatesIconCheckboxPolicy
    category: ICategory
    ballotStyle: IBallotStyle
    contestId: string
    isReview: boolean
    isInvalidWriteIns?: boolean
    isRadioSelection?: boolean
    contest: IContest
    selectedChoicesSum: number
    setSelectedChoicesSum: (num: number) => void
    disableSelect: boolean
}

const showCategoryOnReview = (category: ICategory, questionState?: IDecodedVoteContest) => {
    if (isUndefined(questionState)) {
        return false
    }
    const answersFromCategory = category.candidates.map((candidate) => candidate.id)

    if (!isUndefined(category.header)) {
        answersFromCategory.push(category.header.id)
    }

    return questionState.choices.some(
        (choice) => choice.selected > -1 && answersFromCategory.includes(choice.id)
    )
}

export const AnswersList: React.FC<AnswersListProps> = ({
    title,
    isActive,
    checkableLists,
    checkableCandidates,
    iconCheckboxPolicy,
    category,
    ballotStyle,
    contestId,
    isReview,
    isInvalidWriteIns,
    isRadioSelection,
    contest,
    selectedChoicesSum,
    setSelectedChoicesSum,
    disableSelect,
}) => {
    const categoryAnswerId = category.header?.id || ""
    const selectionState = useAppSelector(
        selectBallotSelectionVoteChoice(ballotStyle.election_id, contestId, categoryAnswerId)
    )
    const questionState = useAppSelector(
        selectBallotSelectionQuestion(ballotStyle.election_id, contestId)
    )
    const dispatch = useAppDispatch()
    const {i18n} = useTranslation()
    let [candidatesOrder, setCandidatesOrder] = useState<Array<string> | null>(null)
    const candidatesOrderType = contest.presentation?.candidates_order
    const isChecked = () => !isUndefined(selectionState) && selectionState.selected > -1
    const setChecked = (value: boolean) => {
        if (isRadioSelection) {
            dispatch(
                resetBallotSelection({
                    ballotStyle,
                    force: true,
                    contestId: contest.id,
                })
            )
        }

        return (
            isActive &&
            dispatch(
                setBallotSelectionVoteChoice({
                    ballotStyle,
                    contestId,
                    voteChoice: {
                        id: categoryAnswerId,
                        selected: value ? 0 : -1,
                    },
                })
            )
        )
    }

    if (isReview && !showCategoryOnReview(category, questionState)) {
        return null
    }

    if (null === candidatesOrder) {
        setCandidatesOrder(
            sortCandidatesInContest(category.candidates, candidatesOrderType, true).map((c) => c.id)
        )
    }

    const categoryCandidatesMap = keyBy(category.candidates, "id")
    let listPresentation = contest.presentation?.types_presentation?.[title] ?? {
        name: title,
    }
    listPresentation.name = title
    let subtypesPresentation = Object.entries(listPresentation.subtypes_presentation ?? {}).map(
        ([key, value]) => {
            value.name = key
            value.sort_order = value.sort_order ?? 0
            return value
        }
    )

    let sortedSubtypes = sortBy(subtypesPresentation, ["sort_order"])

    return (
        <CandidatesList
            title={translate(listPresentation, "name", i18n.language) ?? title}
            isActive={!isReview && isActive}
            isCheckable={checkableLists}
            checked={isChecked()}
            setChecked={setChecked}
        >
            {sortedSubtypes.map((subtypePresentation) => {
                let subtypeCandidates =
                    candidatesOrder
                        ?.map((id) => categoryCandidatesMap[id])
                        .filter(
                            (candidate) =>
                                subtypePresentation.name === candidate.presentation?.subtype
                        ) ?? []

                let subtypeCandidateIds = subtypeCandidates.map((candidate) => candidate.id)
                const hasSelectedAnswer = questionState?.choices.some(
                    (choice) => choice.selected > -1 && subtypeCandidateIds.includes(choice.id)
                )

                if (0 === subtypeCandidates.length || (isReview && !hasSelectedAnswer)) {
                    return null
                }
                return (
                    <>
                        <b>{translate(subtypePresentation, "name", i18n.language)}</b>
                        {subtypeCandidates.map((candidate, candidateIndex) => (
                            <Answer
                                ballotStyle={ballotStyle}
                                answer={candidate}
                                contestId={contestId}
                                key={candidateIndex}
                                index={candidateIndex}
                                hasCategory={true}
                                isActive={!isReview && checkableCandidates}
                                isReview={isReview}
                                isInvalidVote={false}
                                isInvalidWriteIns={isInvalidWriteIns}
                                contest={contest}
                                selectedChoicesSum={selectedChoicesSum}
                                setSelectedChoicesSum={setSelectedChoicesSum}
                                disableSelect={disableSelect}
                                iconCheckboxPolicy={iconCheckboxPolicy}
                            />
                        ))}
                    </>
                )
            })}
            {candidatesOrder
                ?.map((id) => categoryCandidatesMap[id])
                .filter((candidate) => !candidate.presentation?.subtype)
                .map((candidate, candidateIndex) => (
                    <Answer
                        ballotStyle={ballotStyle}
                        answer={candidate}
                        contestId={contestId}
                        key={candidateIndex}
                        index={candidateIndex}
                        hasCategory={true}
                        isActive={!isReview && checkableCandidates}
                        isReview={isReview}
                        isInvalidVote={false}
                        isInvalidWriteIns={isInvalidWriteIns}
                        contest={contest}
                        selectedChoicesSum={selectedChoicesSum}
                        setSelectedChoicesSum={setSelectedChoicesSum}
                        disableSelect={disableSelect}
                        iconCheckboxPolicy={iconCheckboxPolicy}
                    />
                ))}
        </CandidatesList>
    )
}
