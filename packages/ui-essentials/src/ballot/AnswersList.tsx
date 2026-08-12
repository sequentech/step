// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import CandidatesList from "../components/CandidatesList/CandidatesList"
import {
    IDecodedVoteContest,
    isUndefined,
    IContest,
    translate,
    keyBy,
    ECollapsibleLists,
    showCategoryOnReview,
    isCategoryListSelected,
} from "@sequentech/ui-core"
import {Answer} from "./Answer"
import {useBallotEngine} from "./engine"
import {useBallotSelection} from "./selection"
import {ICategory} from "@sequentech/ui-core"
import {IBallotStyle} from "./types"
import {useTranslation} from "react-i18next"
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
    explicitBlank: boolean
    setExplicitBlank: (value: boolean) => void
    setIsTouched: (value: boolean) => void
    externalExpanded?: boolean
    onExpandedChange?: (expanded: boolean) => void
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
    explicitBlank,
    setExplicitBlank,
    setIsTouched,
    externalExpanded,
    onExpandedChange,
}) => {
    const categoryAnswerId = category.header?.id || ""
    const engine = useBallotEngine()
    const selection = useBallotSelection()
    const selectionState = selection.choice(ballotStyle, contestId, categoryAnswerId)
    const questionState = selection.contest(ballotStyle, contestId)
    const {i18n, t} = useTranslation()
    let [candidatesOrder, setCandidatesOrder] = useState<Array<string> | null>(null)
    const candidatesOrderType = contest.presentation?.candidates_order
    const collapsibleListsPolicy =
        contest.presentation?.collapsible_lists ?? ECollapsibleLists.DISABLED
    const isCollapsible = collapsibleListsPolicy !== ECollapsibleLists.DISABLED
    const defaultExpanded = collapsibleListsPolicy !== ECollapsibleLists.ENABLED_COLLAPSED
    const collapseToggleAriaLabel = t("candidatesList.collapseToggle", {listTitle: title})
    const showCandidatesLabel = t("candidatesList.showCandidates")
    const hideCandidatesLabel = t("candidatesList.hideCandidates")
    const categoryCandidateIds = new Set(category.candidates.map((candidate) => candidate.id))
    const selectedCandidatesCount =
        questionState?.choices.filter((choice) => {
            return choice.selected > -1 && categoryCandidateIds.has(choice.id)
        }).length ?? 0
    const selectedCandidatesLabel =
        !isReview && selectedCandidatesCount > 0
            ? t(
                  selectedCandidatesCount === 1
                      ? "candidatesList.selectedCandidate"
                      : "candidatesList.selectedCandidates",
                  {count: selectedCandidatesCount}
              )
            : undefined

    const isChecked = () => !isUndefined(selectionState) && selectionState.selected > -1
    const isListSelectedOnReview =
        isReview && isCategoryListSelected(category, questionState?.choices ?? [])
    const setChecked = (value: boolean): void => {
        if (isRadioSelection) {
            selection.reset({
                ballotStyle,
                force: true,
                contestId: contest.id,
            })
        }

        // `isActive &&` was wrapped around a `dispatch(…)` whose return value was
        // then returned from here and discarded by the caller. Kept as a guard and
        // dropped as an expression: `setChoice` returns nothing, so returning the
        // conjunction would hand back `false` or `undefined` to a prop typed
        // `(value: boolean) => void`.
        if (!isActive) {
            return
        }
        selection.setChoice({
            ballotStyle,
            contestId,
            voteChoice: {
                id: categoryAnswerId,
                selected: value ? 0 : -1,
            },
        })
    }

    if (isReview && !showCategoryOnReview(category, questionState)) {
        return null
    }

    if (null === candidatesOrder) {
        setCandidatesOrder(
            engine
                .sortCandidatesInContest(category.candidates, candidatesOrderType, true)
                .map((c) => c.id)
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

    // `sortBy` from lodash, previously. A shared package taking a utility
    // dependency for one comparison is a dependency every consumer installs, and
    // `sort` mutates, so the list is copied first — which `sortBy` did for free
    // and is the only reason this is two lines rather than one.
    let sortedSubtypes = [...subtypesPresentation].sort(
        (left, right) => (left.sort_order ?? 0) - (right.sort_order ?? 0)
    )

    const shouldDisableList = disableSelect && !isChecked()

    return (
        <CandidatesList
            title={translate(listPresentation, "name", i18n.language) ?? title}
            isActive={!isReview && isActive}
            isCheckable={checkableLists}
            checked={isChecked()}
            setChecked={setChecked}
            shouldDisable={shouldDisableList}
            isCollapsible={!isReview && isCollapsible}
            defaultExpanded={defaultExpanded}
            collapseToggleAriaLabel={collapseToggleAriaLabel}
            showCandidatesLabel={showCandidatesLabel}
            hideCandidatesLabel={hideCandidatesLabel}
            selectedCandidatesLabel={selectedCandidatesLabel}
            externalExpanded={!isReview && isCollapsible ? externalExpanded : undefined}
            onExpandedChange={!isReview && isCollapsible ? onExpandedChange : undefined}
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

                if (
                    0 === subtypeCandidates.length ||
                    (isReview && !hasSelectedAnswer && !isListSelectedOnReview)
                ) {
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
                                isSelectable={!isReview && checkableCandidates}
                                isReview={isReview}
                                isInvalidVote={false}
                                isInvalidWriteIns={isInvalidWriteIns}
                                contest={contest}
                                selectedChoicesSum={selectedChoicesSum}
                                setSelectedChoicesSum={setSelectedChoicesSum}
                                disableSelect={disableSelect}
                                iconCheckboxPolicy={iconCheckboxPolicy}
                                explicitBlank={explicitBlank}
                                setExplicitBlank={setExplicitBlank}
                                setIsTouched={setIsTouched}
                                showWhenListSelected={isListSelectedOnReview}
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
                        isSelectable={!isReview && checkableCandidates}
                        isReview={isReview}
                        isInvalidVote={false}
                        isInvalidWriteIns={isInvalidWriteIns}
                        contest={contest}
                        selectedChoicesSum={selectedChoicesSum}
                        setSelectedChoicesSum={setSelectedChoicesSum}
                        disableSelect={disableSelect}
                        iconCheckboxPolicy={iconCheckboxPolicy}
                        explicitBlank={explicitBlank}
                        setExplicitBlank={setExplicitBlank}
                        setIsTouched={setIsTouched}
                        showWhenListSelected={isListSelectedOnReview}
                    />
                ))}
        </CandidatesList>
    )
}
