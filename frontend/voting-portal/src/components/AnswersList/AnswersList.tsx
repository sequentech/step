// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {CandidatesList, isUndefined} from "@sequentech/ui-essentials"
import {IDecodedVoteQuestion, IElectionDTO} from "sequent-core"
import {Answer} from "../Answer/Answer"
import {useAppDispatch, useAppSelector} from "../../store/hooks"
import {
    selectBallotSelectionQuestion,
    selectBallotSelectionVoteChoice,
    setBallotSelectionVoteChoice,
} from "../../store/ballotSelections/ballotSelectionsSlice"
import {ICategory} from "../../services/CategoryService"

export interface AnswersListProps {
    title: string
    isActive: boolean
    checkableLists: boolean
    checkableCandidates: boolean
    category: ICategory
    election: IElectionDTO
    questionIndex: number
    isReview: boolean
}

const showCategoryOnReview = (category: ICategory, questionState?: IDecodedVoteQuestion) => {
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
    election,
    questionIndex,
    isReview,
}) => {
    const categoryAnswerId = category.header?.id || -1
    const selectionState = useAppSelector(
        selectBallotSelectionVoteChoice(election.id, questionIndex, categoryAnswerId)
    )
    const questionState = useAppSelector(selectBallotSelectionQuestion(election.id, questionIndex))
    const dispatch = useAppDispatch()
    const isChecked = () => !isUndefined(selectionState) && selectionState.selected > -1
    const setChecked = (value: boolean) =>
        isActive &&
        dispatch(
            setBallotSelectionVoteChoice({
                election,
                questionIndex,
                voteChoice: {
                    id: categoryAnswerId,
                    selected: value ? 0 : -1,
                },
            })
        )
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
                    election={election}
                    answer={candidate}
                    questionIndex={questionIndex}
                    key={candidateIndex}
                    hasCategory={true}
                    isActive={!isReview && checkableCandidates}
                    isReview={isReview}
                />
            ))}
        </CandidatesList>
    )
}
