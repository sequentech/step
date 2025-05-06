// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import {sortBy} from "lodash"

import {IDecodedVoteContest, isUndefined, IContest, translate, keyBy} from "@sequentech/ui-core"
import {sortCandidatesInContest, ECandidatesIconCheckboxPolicy} from "@sequentech/ui-core"
import {IBallotStyle as IElectionDTO} from "@sequentech/ui-core"

import {ICategory} from "../../services/CategoryService"
import {CandidatesDisplay} from "../CandidatesDisplay/CandidatesDisplay"
import CandidatesList from "../CandidatesList/CandidatesList"

interface IBallotStyle {
    id: string
    election_id: string
    election_event_id: string
    tenant_id: string
    ballot_eml: IElectionDTO
    ballot_signature?: string | null
    created_at: string
    area_id?: string | null
    annotations?: string | null
    labels?: string | null
    last_updated_at: string
}

export interface CandidatesDisplayListProps {
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
    onResetBallotSelection?: (action: any) => any
    onSetBallotSelectionBlankVote?: (action: any) => any
    onSetBallotSelectionInvalidVote?: (action: any) => any
    onSetBallotSelectionVoteChoice?: (action: any) => any
    questionPlaintext?: IDecodedVoteContest
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

export const CandidatesDisplayList: React.FC<CandidatesDisplayListProps> = ({
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
    onResetBallotSelection,
    onSetBallotSelectionBlankVote,
    onSetBallotSelectionInvalidVote,
    onSetBallotSelectionVoteChoice,
    questionPlaintext,
}) => {
    const categoryAnswerId = category.header?.id || ""
    const selectionState = questionPlaintext?.choices.find((c) => c.id === categoryAnswerId)

    const {i18n} = useTranslation()
    let [candidatesOrder, setCandidatesOrder] = useState<Array<string> | null>(null)
    const candidatesOrderType = contest.presentation?.candidates_order
    const isChecked = () => !isUndefined(selectionState) && selectionState.selected > -1
    const setChecked = (value: boolean) => {
        if (isRadioSelection) {
            onResetBallotSelection?.({
                ballotStyle,
                force: true,
                contestId: contest.id,
            })
        }

        return (
            isActive &&
            onSetBallotSelectionVoteChoice?.({
                ballotStyle,
                contestId,
                voteChoice: {
                    id: categoryAnswerId,
                    selected: value ? 0 : -1,
                },
            })
        )
    }

    if (isReview && !showCategoryOnReview(category, questionPlaintext)) {
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
                const hasSelectedAnswer = questionPlaintext?.choices.some(
                    (choice) => choice.selected > -1 && subtypeCandidateIds.includes(choice.id)
                )

                if (0 === subtypeCandidates.length || (isReview && !hasSelectedAnswer)) {
                    return null
                }
                return (
                    <>
                        <b>{translate(subtypePresentation, "name", i18n.language)}</b>
                        {subtypeCandidates.map((candidate, candidateIndex) => (
                            <CandidatesDisplay
                                ballotStyle={ballotStyle}
                                answer={candidate}
                                contestId={contestId}
                                key={candidateIndex}
                                index={candidateIndex}
                                hasCategory={true}
                                isActive={!isReview && checkableCandidates}
                                isReview={isReview}
                                isInvalidWriteIns={isInvalidWriteIns}
                                contest={contest}
                                selectedChoicesSum={selectedChoicesSum}
                                setSelectedChoicesSum={setSelectedChoicesSum}
                                disableSelect={disableSelect}
                                iconCheckboxPolicy={iconCheckboxPolicy}
                                onResetBallotSelection={onResetBallotSelection}
                                onSetBallotSelectionBlankVote={onSetBallotSelectionBlankVote}
                                onSetBallotSelectionInvalidVote={onSetBallotSelectionInvalidVote}
                                onSetBallotSelectionVoteChoice={onSetBallotSelectionVoteChoice}
                            />
                        ))}
                    </>
                )
            })}
            {candidatesOrder
                ?.map((id) => categoryCandidatesMap[id])
                .filter((candidate) => !candidate.presentation?.subtype)
                .map((candidate, candidateIndex) => (
                    <CandidatesDisplay
                        ballotStyle={ballotStyle}
                        answer={candidate}
                        contestId={contestId}
                        key={candidateIndex}
                        index={candidateIndex}
                        hasCategory={true}
                        isActive={!isReview && checkableCandidates}
                        isReview={isReview}
                        isInvalidWriteIns={isInvalidWriteIns}
                        contest={contest}
                        selectedChoicesSum={selectedChoicesSum}
                        setSelectedChoicesSum={setSelectedChoicesSum}
                        disableSelect={disableSelect}
                        iconCheckboxPolicy={iconCheckboxPolicy}
                        onResetBallotSelection={onResetBallotSelection}
                        onSetBallotSelectionBlankVote={onSetBallotSelectionBlankVote}
                        onSetBallotSelectionInvalidVote={onSetBallotSelectionInvalidVote}
                        onSetBallotSelectionVoteChoice={onSetBallotSelectionVoteChoice}
                    />
                ))}
        </CandidatesList>
    )
}
