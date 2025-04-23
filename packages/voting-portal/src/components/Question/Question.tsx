// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {
    stringToHtml,
    splitList,
    keyBy,
    translate,
    IContest,
    CandidatesOrder,
    EOverVotePolicy,
    ECandidatesIconCheckboxPolicy,
    BallotSelection,
} from "@sequentech/ui-core"
import {
    checkIsExplicitBlankVote,
    checkIsInvalidVote,
    checkIsRadioSelection,
    checkPositionIsTop,
    checkShuffleCategories,
    checkShuffleCategoryList,
    getCheckableOptions,
    checkAllowWriteIns,
    checkIsWriteIn,
} from "../../services/ElectionConfigService"
import {
    CategoriesMap,
    categorizeCandidates,
    getShuffledCategories,
} from "../../services/CategoryService"
import {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {useTranslation} from "react-i18next"
import {IDecodedVoteContest} from "@sequentech/ui-core"
import {useAppSelector} from "../../store/hooks"
import {selectBallotSelectionQuestion} from "../../store/ballotSelections/ballotSelectionsSlice"
import {sortCandidatesInContest, checkIsBlank} from "@sequentech/ui-core"
import {QuestionUI} from "./QuestionUI"

export interface IQuestionProps {
    ballotStyle: IBallotStyle
    question: IContest
    isReview: boolean
    setDisableNext?: (value: boolean) => void
    setDecodedContests: (input: IDecodedVoteContest) => void
    errorSelectionState: BallotSelection
}

export const Question: React.FC<IQuestionProps> = ({
    ballotStyle,
    question,
    isReview,
    setDisableNext,
    setDecodedContests,
    errorSelectionState,
}) => {
    // State and Redux selectors
    const {i18n} = useTranslation()
    let [candidatesOrder, setCandidatesOrder] = useState<Array<string> | null>(null)
    let [categoriesMapOrder, setCategoriesMapOrder] = useState<CategoriesMap | null>(null)
    let [isInvalidWriteIns, setIsInvalidWriteIns] = useState(false)
    let [selectedChoicesSum, setSelectedChoicesSum] = useState(0)
    let [disableSelect, setDisableSelect] = useState(false)
    let {invalidOrBlankCandidates, noCategoryCandidates, categoriesMap} =
        categorizeCandidates(question)
    let hasBlankCandidate = invalidOrBlankCandidates.some((candidate) =>
        checkIsExplicitBlankVote(candidate)
    )
    const contestState = useAppSelector(
        selectBallotSelectionQuestion(ballotStyle.election_id, question.id)
    )

    // Computed properties
    const {checkableLists, checkableCandidates} = getCheckableOptions(question)
    let [invalidBottomCandidates, invalidTopCandidates] = splitList(
        invalidOrBlankCandidates,
        checkPositionIsTop
    )
    let hasWriteIns = checkAllowWriteIns(question) && !!question.candidates.find(checkIsWriteIn)
    const maxVotesNum = question.max_votes
    const overVoteDisableMode =
        question.presentation?.over_vote_policy === EOverVotePolicy.NOT_ALLOWED_WITH_MSG_AND_DISABLE
    const iconCheckboxPolicy =
        question.presentation?.candidates_icon_checkbox_policy ??
        ECandidatesIconCheckboxPolicy.SQUARE_CHECKBOX
    const columnCount = question.presentation?.columns ?? 1
    const isRadioSelection = checkIsRadioSelection(question)
    const isBlank = isReview && contestState && checkIsBlank(contestState)

    // Effects
    useEffect(() => {
        // Calculating the number of selected candidates
        let selectedChoicesCount = 0
        contestState?.choices.forEach((choice) => {
            choice.selected === 0 && selectedChoicesCount++
        })
        setSelectedChoicesSum(selectedChoicesCount)
    }, [contestState])

    useEffect(() => {
        if (overVoteDisableMode) {
            if (selectedChoicesSum >= maxVotesNum) {
                setDisableSelect(true)
            } else {
                setDisableSelect(false)
            }
        }
    }, [selectedChoicesSum, maxVotesNum, overVoteDisableMode])

    // Do the shuffling
    const candidatesOrderType = question.presentation?.candidates_order
    const shuffleCategories = checkShuffleCategories(question)
    const shuffleCategoryList = checkShuffleCategoryList(question)
    if (null === categoriesMapOrder) {
        setCategoriesMapOrder(
            getShuffledCategories(
                categoriesMap,
                candidatesOrderType === CandidatesOrder.RANDOM,
                shuffleCategories,
                shuffleCategoryList,
                question.presentation?.types_presentation
            )
        )
    }

    if (null === candidatesOrder) {
        setCandidatesOrder(
            sortCandidatesInContest(noCategoryCandidates, candidatesOrderType, true).map(
                (c) => c.id
            )
        )
    }

    const noCategoryCandidatesMap = keyBy(noCategoryCandidates, "id")

    const onSetIsInvalidWriteIns = (value: boolean) => {
        setIsInvalidWriteIns(value)
        setDisableNext?.(value)
    }

    // Render the UI component with all prepared data
    return (
        <QuestionUI
            title={translate(question, "name", i18n.language) || ""}
            description={stringToHtml(translate(question, "description", i18n.language) || "")}
            isReview={isReview}
            isBlank={!!isBlank}
            columnCount={columnCount}
            hasWriteIns={hasWriteIns}
            isInvalidWriteIns={isInvalidWriteIns}
            setIsInvalidWriteIns={onSetIsInvalidWriteIns}
            ballotStyle={ballotStyle}
            question={question}
            candidatesOrder={candidatesOrder}
            noCategoryCandidatesMap={noCategoryCandidatesMap}
            categoriesMapOrder={categoriesMapOrder}
            checkableLists={checkableLists}
            checkableCandidates={checkableCandidates}
            selectedChoicesSum={selectedChoicesSum}
            setSelectedChoicesSum={setSelectedChoicesSum}
            disableSelect={disableSelect}
            isRadioSelection={isRadioSelection}
            iconCheckboxPolicy={iconCheckboxPolicy}
            setDecodedContests={setDecodedContests}
            errorSelectionState={errorSelectionState}
        />
    )
}
