// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useState} from "react"
import {Alert, Box, Button} from "@mui/material"
import {
    stringToHtml,
    splitList,
    keyBy,
    translate,
    translateFromPresentation,
    isAcclaimedContest,
    IContest,
    CandidatesOrder,
    EOverVotePolicy,
    ECandidatesIconCheckboxPolicy,
    BallotSelection,
    ECollapsibleLists,
} from "@sequentech/ui-core"
import {theme, BlankAnswer} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"
import Typography from "@mui/material/Typography"
import {Answer} from "../Answer/Answer"
import {AnswersList} from "../AnswersList/AnswersList"
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
import {InvalidErrorsList} from "../InvalidErrorsList/InvalidErrorsList"
import {useTranslation} from "react-i18next"
import {IDecodedVoteContest, IInvalidPlaintextError} from "@sequentech/ui-core"
import {useAppSelector} from "../../store/hooks"
import {selectBallotSelectionQuestion} from "../../store/ballotSelections/ballotSelectionsSlice"
import {sortCandidatesInContest, checkIsBlank} from "@sequentech/ui-core"
import {provideBallotService} from "../../services/BallotService"
import {faAngleDown, faAngleRight} from "@fortawesome/free-solid-svg-icons"
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome"

const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
    align-items: center;
`

const CandidatesWrapper = styled("fieldset")`
    display: flex;
    flex-direction: column;
    border: 0;
    margin: 0;
    padding: 0;
    min-inline-size: 0;

    ul + ul {
        margin: 12px 0;
    }
`

const CandidateListsWrapper = styled(Box)<{columncount: number}>`
    display: flex;
    flex-direction: ${({columncount}) => (columncount === 1 ? "column" : "row")};
    gap: 12px;
    margin: 12px 0 0 0;

    /* If there's only one column, we want to make sure the candidate lists take the full width of the container */
    ${({columncount}) => (columncount === 1 ? `.candidates-list { width: initial; }` : "")}

    @media (max-width: ${({theme}) => theme.breakpoints.values.md}px) {
        flex-direction: column;

        .candidates-list {
            width: initial;
        }
    }
`

const CandidatesSingleWrapper = styled("ul")<{columnCount: number}>`
    list-style: none;
    padding-inline-start: 0;
    column-gap: 0;
    margin: 0;

    @media (min-width: ${({theme}) => theme.breakpoints.values.lg}px) {
        column-count: ${(data) => data.columnCount};
    }

    li + li {
        margin-top: 12px;
    }
`

const InvalidBlankWrapper = styled("ul")<{columnCount: number}>`
    list-style: none;
    padding-inline-start: 0;
    column-gap: 0;
    margin: 0;

    @media (min-width: ${({theme}) => theme.breakpoints.values.lg}px) {
        column-count: ${(data) => data.columnCount};
    }

    li + li {
        margin-top: 12px;
    }
`
export interface IQuestionProps {
    ballotStyle: IBallotStyle
    question: IContest
    isReview: boolean
    setDisableNext?: (value: boolean) => void
    setDecodedContests: (input: IDecodedVoteContest) => void
    errorSelectionState: BallotSelection
    isDeclineToVote?: boolean
    isBlankBallot?: boolean
}

export const Question: React.FC<IQuestionProps> = ({
    ballotStyle,
    question,
    isReview,
    setDisableNext,
    setDecodedContests,
    errorSelectionState,
    isDeclineToVote,
    isBlankBallot,
}) => {
    // THIS IS A CONTEST COMPONENT
    const {i18n, t} = useTranslation()
    const {isPreferential} = provideBallotService()
    const isPreferentialVote = isPreferential(question.counting_algorithm)
    let [candidatesOrder, setCandidatesOrder] = useState<Array<string> | null>(null)
    const [explicitBlank, setExplicitBlank] = useState<boolean>(false)
    let [categoriesMapOrder, setCategoriesMapOrder] = useState<CategoriesMap | null>(null)
    let [isInvalidWriteIns, setIsInvalidWriteIns] = useState(false)
    let [selectedChoicesSum, setSelectedChoicesSum] = useState(0)
    let [disableSelect, setDisableSelect] = useState(false)
    let {invalidOrBlankCandidates, noCategoryCandidates, categoriesMap} =
        categorizeCandidates(question)
    // An acclaimed contest is display-only: nothing can be selected, so it
    // offers no explicit blank or invalid options, reports no selection
    // errors, and is not part of a declined or blank ballot either.
    const isAcclaimed = isAcclaimedContest(question)
    const markerCandidates = isAcclaimed ? [] : invalidOrBlankCandidates
    const defaultLanguageCode =
        ballotStyle.ballot_eml.election_presentation?.language_conf?.default_language_code ??
        ballotStyle.ballot_eml.election_event_presentation?.language_conf?.default_language_code
    const showDeclineToVote = !!isDeclineToVote && !isAcclaimed
    const showBlankBallot = !!isBlankBallot && !isAcclaimed
    const [isTouched, setIsTouched] = useState(isReview)
    const contestState = useAppSelector(
        selectBallotSelectionQuestion(ballotStyle.election_id, question.id)
    )
    const {checkableLists, checkableCandidates} = getCheckableOptions(question)
    const explicitBlankCandidateIds = useMemo(
        () =>
            new Set(
                question.candidates
                    .filter((candidate) => checkIsExplicitBlankVote(candidate))
                    .map((candidate) => candidate.id)
            ),
        [question.candidates]
    )

    const collapsibleListsPolicy =
        question.presentation?.collapsible_lists ?? ECollapsibleLists.DISABLED
    const isCollapsible = collapsibleListsPolicy !== ECollapsibleLists.DISABLED
    const defaultAllExpanded = collapsibleListsPolicy !== ECollapsibleLists.ENABLED_COLLAPSED
    const [expandedStates, setExpandedStates] = useState<Record<string, boolean>>({})

    const getExpanded = (key: string): boolean =>
        key in expandedStates ? expandedStates[key] : defaultAllExpanded

    const allCollapsed =
        !!categoriesMapOrder &&
        Object.keys(categoriesMapOrder).length > 0 &&
        Object.keys(categoriesMapOrder).every((k) => !getExpanded(k))

    const handleToggleAll = () => {
        if (!categoriesMapOrder) return
        const targetExpanded = allCollapsed
        const newState: Record<string, boolean> = {}
        Object.keys(categoriesMapOrder).forEach((k) => {
            newState[k] = targetExpanded
        })
        setExpandedStates(newState)
    }

    // do the shuffling
    const candidatesOrderType = question.presentation?.candidates_order

    let [invalidBottomCandidatesUnsorted, invalidTopCandidatesUnsorted] = splitList(
        markerCandidates,
        checkPositionIsTop
    )

    // Sort invalid/blank candidates within their top/bottom blocks
    let invalidBottomCandidates = sortCandidatesInContest(
        invalidBottomCandidatesUnsorted,
        candidatesOrderType,
        true
    )
    let invalidTopCandidates = sortCandidatesInContest(
        invalidTopCandidatesUnsorted,
        candidatesOrderType,
        true
    )

    let hasWriteIns = checkAllowWriteIns(question) && !!question.candidates.find(checkIsWriteIn)

    useEffect(() => {
        // Calculating the number of selected candidates
        let selectedChoicesCount = contestState?.is_explicit_invalid ? 1 : 0
        contestState?.choices.forEach((choice) => {
            choice.selected >= 0 && selectedChoicesCount++
        })
        setSelectedChoicesSum(selectedChoicesCount)
    }, [contestState])

    useEffect(() => {
        setExplicitBlank(
            !!contestState?.choices.some(
                (choice) => explicitBlankCandidateIds.has(choice.id) && choice.selected > -1
            )
        )
    }, [contestState, explicitBlankCandidateIds])

    const maxVotesNum = question.max_votes
    const overVoteDisableMode =
        question.presentation?.over_vote_policy === EOverVotePolicy.NOT_ALLOWED_WITH_MSG_AND_DISABLE
    const iconCheckboxPolicy =
        question.presentation?.candidates_icon_checkbox_policy ??
        ECandidatesIconCheckboxPolicy.SQUARE_CHECKBOX
    const columnCount = question.presentation?.columns ?? 1

    useEffect(() => {
        if (overVoteDisableMode) {
            if (selectedChoicesSum >= maxVotesNum) {
                setDisableSelect(true)
            } else {
                setDisableSelect(false)
            }
        }
    }, [selectedChoicesSum])

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
        let sortedCandidates = sortCandidatesInContest(
            noCategoryCandidates,
            candidatesOrderType,
            true
        )

        if (isReview && isPreferentialVote && contestState) {
            const choicesById = keyBy(contestState.choices, "id")
            sortedCandidates = [...sortedCandidates].sort((a, b) => {
                const aRank = choicesById[a.id]?.selected ?? -1
                const bRank = choicesById[b.id]?.selected ?? -1
                if (aRank === -1 && bRank === -1) return 0
                if (aRank === -1) return 1
                if (bRank === -1) return -1
                return aRank - bRank
            })
        }

        setCandidatesOrder(sortedCandidates.map((c) => c.id))
    }

    const noCategoryCandidatesMap = keyBy(noCategoryCandidates, "id")

    const onSetIsInvalidWriteIns = (value: boolean) => {
        setIsInvalidWriteIns(value)
        setDisableNext?.(value)
    }

    // when isRadioChecked is true, clicking on another option works as a radio button:
    // it deselects the previously selected option to select the new one
    const isRadioSelection = checkIsRadioSelection(question)
    const isBlank = !isAcclaimed && isReview && contestState && checkIsBlank(contestState)

    return (
        <Box component="section" aria-labelledby={`contest-${question.id}-title`}>
            <StyledTitle
                className="contest-title"
                variant="h5"
                data-min={question.min_votes}
                data-max={question.max_votes}
                id={`contest-${question.id}-title`}
            >
                <Box component="span" sx={{flexGrow: 1}}>
                    {translate(question, "name", i18n.language) || ""}
                </Box>
                {isCollapsible &&
                !isReview &&
                !!categoriesMapOrder &&
                Object.keys(categoriesMapOrder).length ? (
                    <Button
                        variant="secondary"
                        sx={{flexShrink: 0, minHeight: "unset", fontSize: "14px"}}
                        startIcon={
                            <FontAwesomeIcon icon={allCollapsed ? faAngleRight : faAngleDown} />
                        }
                        onClick={handleToggleAll}
                    >
                        {allCollapsed
                            ? t("candidatesList.expandAll")
                            : t("candidatesList.collapseAll")}
                    </Button>
                ) : null}
            </StyledTitle>
            {question.description || question.description_i18n?.[i18n.language] ? (
                <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                    {stringToHtml(translate(question, "description", i18n.language) || "")}
                </Typography>
            ) : null}
            {isAcclaimed ? (
                <Alert severity="info" className="contest-acclamation">
                    {stringToHtml(
                        translateFromPresentation(
                            question,
                            "acclamation_description",
                            i18n.language,
                            {defaultLanguageCode}
                        ) || t("contest.acclamation.description")
                    )}
                </Alert>
            ) : null}
            {showDeclineToVote ? (
                <InvalidBlankWrapper className="candidates-review-decline" columnCount={1}>
                    <BlankAnswer title={t("reviewScreen.declineToVote")} />
                </InvalidBlankWrapper>
            ) : showBlankBallot ? (
                <InvalidBlankWrapper className="candidates-review-blank-ballot" columnCount={1}>
                    <BlankAnswer title={t("reviewScreen.blankBallot")} />
                </InvalidBlankWrapper>
            ) : (
                <>
                    {isAcclaimed ? null : (
                        <InvalidErrorsList
                            ballotStyle={ballotStyle}
                            question={question}
                            hasWriteIns={hasWriteIns}
                            isInvalidWriteIns={isInvalidWriteIns}
                            setIsInvalidWriteIns={onSetIsInvalidWriteIns}
                            setDecodedContests={setDecodedContests}
                            isReview={isReview}
                            errorSelectionState={errorSelectionState}
                            isTouched={isTouched}
                            setIsTouched={setIsTouched}
                        />
                    )}
                    {isBlank ? (
                        <InvalidBlankWrapper className="candidates-review-blank" columnCount={1}>
                            <BlankAnswer />{" "}
                        </InvalidBlankWrapper>
                    ) : null}
                    <CandidatesWrapper className="candidates-container">
                        <Box
                            className="candidates-legend"
                            component="legend"
                            sx={{
                                position: "absolute",
                                width: 0,
                                height: 0,
                                overflow: "hidden",
                                clip: "rect(0 0 0 0)",
                            }}
                        >
                            {translate(question, "name", i18n.language) || ""}
                        </Box>
                        {invalidTopCandidates.length ? (
                            <InvalidBlankWrapper
                                className="candidates-top-blank-invalid"
                                columnCount={1}
                            >
                                {invalidTopCandidates.map((answer, answerIndex) => (
                                    <Answer
                                        ballotStyle={ballotStyle}
                                        answer={answer}
                                        contestId={question.id}
                                        key={answerIndex}
                                        index={answerIndex}
                                        isSelectable={!isReview}
                                        isReview={isReview}
                                        isExplicitBlankVote={checkIsExplicitBlankVote(answer)}
                                        isRadioSelection={isRadioSelection}
                                        contest={question}
                                        selectedChoicesSum={selectedChoicesSum}
                                        setSelectedChoicesSum={setSelectedChoicesSum}
                                        disableSelect={disableSelect}
                                        iconCheckboxPolicy={iconCheckboxPolicy}
                                        explicitBlank={explicitBlank}
                                        setExplicitBlank={setExplicitBlank}
                                        setIsTouched={setIsTouched}
                                    />
                                ))}
                            </InvalidBlankWrapper>
                        ) : null}
                        {!!categoriesMapOrder && Object.keys(categoriesMapOrder)?.length ? (
                            <CandidateListsWrapper
                                className="candidates-lists-container"
                                columncount={columnCount}
                            >
                                {Object.entries(categoriesMapOrder).map(
                                    ([categoryName, category], categoryIndex) => (
                                        <AnswersList
                                            key={categoryIndex}
                                            title={categoryName}
                                            isActive={true}
                                            checkableLists={checkableLists}
                                            checkableCandidates={checkableCandidates}
                                            category={category}
                                            ballotStyle={ballotStyle}
                                            contestId={question.id}
                                            isReview={isReview}
                                            isInvalidWriteIns={isInvalidWriteIns}
                                            isRadioSelection={isRadioSelection}
                                            contest={question}
                                            selectedChoicesSum={selectedChoicesSum}
                                            setSelectedChoicesSum={setSelectedChoicesSum}
                                            disableSelect={disableSelect}
                                            iconCheckboxPolicy={iconCheckboxPolicy}
                                            explicitBlank={explicitBlank}
                                            setExplicitBlank={setExplicitBlank}
                                            setIsTouched={setIsTouched}
                                            externalExpanded={getExpanded(categoryName)}
                                            onExpandedChange={(expanded) =>
                                                setExpandedStates((prev) => ({
                                                    ...prev,
                                                    [categoryName]: expanded,
                                                }))
                                            }
                                        />
                                    )
                                )}
                            </CandidateListsWrapper>
                        ) : null}
                        {candidatesOrder?.length ? (
                            <CandidatesSingleWrapper
                                className="candidates-singles-container"
                                columnCount={columnCount}
                            >
                                {candidatesOrder
                                    ?.map((id) => noCategoryCandidatesMap[id])
                                    .map((answer, answerIndex) => (
                                        <Answer
                                            isInvalidWriteIns={isInvalidWriteIns}
                                            ballotStyle={ballotStyle}
                                            answer={answer}
                                            contestId={question.id}
                                            index={answerIndex}
                                            key={answerIndex}
                                            isSelectable={!isReview}
                                            isInvalidVote={false}
                                            isReview={isReview}
                                            isRadioSelection={isRadioSelection}
                                            contest={question}
                                            selectedChoicesSum={selectedChoicesSum}
                                            setSelectedChoicesSum={setSelectedChoicesSum}
                                            disableSelect={disableSelect}
                                            iconCheckboxPolicy={iconCheckboxPolicy}
                                            explicitBlank={explicitBlank}
                                            setExplicitBlank={setExplicitBlank}
                                            setIsTouched={setIsTouched}
                                        />
                                    ))}
                            </CandidatesSingleWrapper>
                        ) : null}
                        {invalidBottomCandidates.length ? (
                            <InvalidBlankWrapper
                                className="candidates-bottom-blank-invalid"
                                columnCount={1}
                            >
                                {invalidBottomCandidates.map((answer, answerIndex) => (
                                    <Answer
                                        ballotStyle={ballotStyle}
                                        answer={answer}
                                        contestId={question.id}
                                        index={answerIndex}
                                        key={answerIndex}
                                        isSelectable={!isReview}
                                        isReview={isReview}
                                        isExplicitBlankVote={checkIsExplicitBlankVote(answer)}
                                        isInvalidWriteIns={false}
                                        isRadioSelection={isRadioSelection}
                                        contest={question}
                                        selectedChoicesSum={selectedChoicesSum}
                                        setSelectedChoicesSum={setSelectedChoicesSum}
                                        disableSelect={disableSelect}
                                        iconCheckboxPolicy={iconCheckboxPolicy}
                                        explicitBlank={explicitBlank}
                                        setExplicitBlank={setExplicitBlank}
                                        setIsTouched={setIsTouched}
                                    />
                                ))}
                            </InvalidBlankWrapper>
                        ) : null}
                    </CandidatesWrapper>
                </>
            )}
        </Box>
    )
}
