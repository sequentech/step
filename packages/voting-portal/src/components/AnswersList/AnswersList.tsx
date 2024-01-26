// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {CandidatesList, isUndefined} from "@sequentech/ui-essentials"
import {IDecodedVoteContest} from "sequent-core"
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

export interface AnswersListProps {
    title: string
    isActive: boolean
    checkableLists: boolean
    checkableCandidates: boolean
    category: ICategory
    ballotStyle: IBallotStyle
    questionIndex: number
    isReview: boolean
    isInvalidWriteIns?: boolean
    isUniqChecked?: boolean
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
    category,
    ballotStyle,
    questionIndex,
    isReview,
    isInvalidWriteIns,
    isUniqChecked,
}) => {
    const categoryAnswerId = category.header?.id || ""
    const selectionState = useAppSelector(
        selectBallotSelectionVoteChoice(ballotStyle.election_id, questionIndex, categoryAnswerId)
    )
    const questionState = useAppSelector(
        selectBallotSelectionQuestion(ballotStyle.election_id, questionIndex)
    )
    const dispatch = useAppDispatch()
    const isChecked = () => !isUndefined(selectionState) && selectionState.selected > -1
    const setChecked = (value: boolean) => {
        if (isUniqChecked) {
            dispatch(
                resetBallotSelection({
                    ballotStyle,
                    force: true,
                    questionIndex,
                })
            )
        }

        return (
            isActive &&
            dispatch(
                setBallotSelectionVoteChoice({
                    ballotStyle,
                    questionIndex,
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

    return (
        <CandidatesList
            title={title}
            isActive={!isReview && isActive}
            isCheckable={checkableLists}
            checked={isChecked()}
            setChecked={setChecked}
        >
            {category.candidates.map((candidate, candidateIndex) => (
                <Answer
                    ballotStyle={ballotStyle}
                    answer={candidate}
                    questionIndex={questionIndex}
                    key={candidateIndex}
                    index={candidateIndex}
                    hasCategory={true}
                    isActive={!isReview && checkableCandidates}
                    isReview={isReview}
                    isInvalidWriteIns={isInvalidWriteIns}
                />
            ))}
        </CandidatesList>
    )
}
