// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {Box} from "@mui/material"
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

const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
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

const CandidateListsWrapper = styled(Box)`
    display: flex;
    flex-direction: row;
    gap: 12px;
    margin: 12px 0 0 0;

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
}

export const Question: React.FC<IQuestionProps> = ({
    ballotStyle,
    question,
    isReview,
    setDisableNext,
    setDecodedContests,
    errorSelectionState,
}) => {
    // THIS IS A CONTEST COMPONENT
    const {i18n} = useTranslation()
    let [candidatesOrder, setCandidatesOrder] = useState<Array<string> | null>(null)
    const [explicitBlank, setExplicitBlank] = useState<boolean>(false)
    let [categoriesMapOrder, setCategoriesMapOrder] = useState<CategoriesMap | null>(null)
    let [isInvalidWriteIns, setIsInvalidWriteIns] = useState(false)
    let [selectedChoicesSum, setSelectedChoicesSum] = useState(0)
    let [disableSelect, setDisableSelect] = useState(false)
    let {invalidOrBlankCandidates, noCategoryCandidates, categoriesMap} =
        categorizeCandidates(question)
    const [isTouched, setIsTouched] = useState(isReview)
    const contestState = useAppSelector(
        selectBallotSelectionQuestion(ballotStyle.election_id, question.id)
    )
    const {checkableLists, checkableCandidates} = getCheckableOptions(question)

    // do the shuffling
    const candidatesOrderType = question.presentation?.candidates_order

    let [invalidBottomCandidatesUnsorted, invalidTopCandidatesUnsorted] = splitList(
        invalidOrBlankCandidates,
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
        let selectedChoicesCount = 0
        contestState?.choices.forEach((choice) => {
            choice.selected === 0 && selectedChoicesCount++
        })
        setSelectedChoicesSum(selectedChoicesCount)
    }, [contestState])

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

    // when isRadioChecked is true, clicking on another option works as a radio button:
    // it deselects the previously selected option to select the new one
    const isRadioSelection = checkIsRadioSelection(question)
    const isBlank = isReview && contestState && checkIsBlank(contestState)

    return (
        <Box component="section" aria-labelledby={`contest-${question.id}-title`}>
            <StyledTitle
                className="contest-title"
                variant="h5"
                data-min={question.min_votes}
                data-max={question.max_votes}
                id={`contest-${question.id}-title`}
            >
                {translate(question, "name", i18n.language) || ""}
            </StyledTitle>
            {question.description || question.description_i18n?.[i18n.language] ? (
                <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                    {stringToHtml(translate(question, "description", i18n.language) || "")}
                </Typography>
            ) : null}
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
                    <InvalidBlankWrapper className="candidates-top-blank-invalid" columnCount={1}>
                        {invalidTopCandidates.map((answer, answerIndex) => (
                            <Answer
                                ballotStyle={ballotStyle}
                                answer={answer}
                                contestId={question.id}
                                key={answerIndex}
                                index={answerIndex}
                                isActive={!isReview}
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
                    <CandidateListsWrapper className="candidates-lists-container">
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
                                    isActive={!isReview}
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
                                isActive={!isReview}
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
        </Box>
    )
}
