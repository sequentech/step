// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useTranslation} from "react-i18next"
import {Box} from "@mui/material"
import {styled} from "@mui/material/styles"
import emotionStyled from "@emotion/styled"
import Typography from "@mui/material/Typography"

import {
    stringToHtml,
    splitList,
    translate,
    IContest,
    CandidatesOrder,
    EOverVotePolicy,
    ECandidatesIconCheckboxPolicy,
    BallotSelection,
    ICandidate,
} from "@sequentech/ui-core"
import {IDecodedVoteContest, IInvalidPlaintextError} from "@sequentech/ui-core"
import {sortCandidatesInContest, checkIsBlank} from "@sequentech/ui-core"
import {theme, BlankAnswer} from "@sequentech/ui-essentials"

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

import {InvalidErrorsList} from "../InvalidErrorsList/InvalidErrorsList"
import {Answer} from "../Answer/Answer"
import {AnswersList} from "../AnswersList/AnswersList"
import {IBallotStyle as IElectionDTO} from "@sequentech/ui-core"

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

const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
`

const CandidatesWrapper = styled(Box)`
    display: flex;
    flex-direction: column;
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

const CandidatesSingleWrapper = emotionStyled.ul<{columnCount: number}>`
    list-style: none;
    margin: 12px 0;
    padding-inline-start: 0;
    column-gap: 0;
    
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
    questionPlaintext?: IDecodedVoteContest
    setDisableNext?: (value: boolean) => void
    setDecodedContests?: (input: IDecodedVoteContest) => void
    errorSelectionState: BallotSelection
    onResetBallotSelection?: (action: any) => any
    onSetBallotSelectionBlankVote?: (action: any) => any
    onSetBallotSelectionInvalidVote?: (action: any) => any
    onSetBallotSelectionVoteChoice?: (action: any) => any
    url?: string
    isVotedState: boolean
}

export const Question: React.FC<IQuestionProps> = ({
    ballotStyle,
    question,
    questionPlaintext,
    isReview,
    setDisableNext,
    setDecodedContests,
    errorSelectionState,
    onResetBallotSelection,
    onSetBallotSelectionBlankVote,
    onSetBallotSelectionInvalidVote,
    onSetBallotSelectionVoteChoice,
    url,
    isVotedState,
}) => {
    // THIS IS A CONTEST COMPONENT
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

    const {checkableLists, checkableCandidates} = getCheckableOptions(question)
    let [invalidBottomCandidates, invalidTopCandidates] = splitList(
        invalidOrBlankCandidates,
        checkPositionIsTop
    )
    let hasWriteIns = checkAllowWriteIns(question) && !!question.candidates.find(checkIsWriteIn)

    useEffect(() => {
        // Calculating the number of selected candidates
        let selectedChoicesCount = 0
        questionPlaintext?.choices.forEach((choice) => {
            choice.selected === 0 && selectedChoicesCount++
        })
        setSelectedChoicesSum(selectedChoicesCount)
    }, [questionPlaintext])

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

    // do the shuffling
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

    /**
     * Lodash-like keyBy implementation - creates an object keyed by the specified property
     * or function result for each item in the array.
     *
     * @param {Array} array - The array to convert to an object
     * @param {String|Function} iteratee - Property name or function to generate keys
     * @returns {Object} - Object with values keyed by iteratee result
     */
    function keyBy(array: any, iteratee: any) {
        // Handle empty arrays
        if (!array || !array.length) {
            return {}
        }

        const result: Record<string, any> = {}
        const isFunction = typeof iteratee === "function"

        // Process each item in the array
        for (let i = 0; i < array.length; i++) {
            const item = array[i]
            // Get the key by calling the function or accessing the property
            const key = isFunction ? iteratee(item) : item[iteratee]

            // Only add defined keys to the result
            if (key !== undefined && key !== null) {
                result[key] = item
            }
        }

        return result
    }

    const selectedAnswers = questionPlaintext?.choices.filter((a) => a.selected > -1)

    const answersById = keyBy(question.candidates, (a: ICandidate) => a.id)

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
    const isBlank = isReview && questionPlaintext && checkIsBlank(questionPlaintext)

    return (
        <Box>
            <StyledTitle
                className="contest-title"
                variant="h5"
                data-min={question.min_votes}
                data-max={question.max_votes}
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
                isVotedState={isVotedState}
            />
            {isBlank ? <BlankAnswer /> : null}
            <CandidatesWrapper className="candidates-container">
                {invalidTopCandidates.map((answer, answerIndex) => (
                    <Answer
                        ballotStyle={ballotStyle}
                        answer={answer}
                        contestId={question.id}
                        key={answerIndex}
                        index={answerIndex}
                        isActive={!isReview}
                        isReview={isReview}
                        isInvalidVote={checkIsInvalidVote(answer)}
                        isExplicitBlankVote={checkIsExplicitBlankVote(answer)}
                        isRadioSelection={isRadioSelection}
                        contest={question}
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
                <CandidateListsWrapper className="candidates-lists-container">
                    {categoriesMapOrder &&
                        Object.entries(categoriesMapOrder).map(
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
                                />
                            )
                        )}
                </CandidateListsWrapper>
                <CandidatesSingleWrapper
                    className="candidates-singles-container"
                    columnCount={columnCount}
                >
                    {isReview
                        ? selectedAnswers?.map((answer, answerIndex) => (
                              <Answer
                                  ballotStyle={ballotStyle}
                                  key={answerIndex}
                                  isInvalidWriteIns={isInvalidWriteIns}
                                  answer={answersById[answer.id]}
                                  questionPlaintext={questionPlaintext}
                                  writeInValue={answer.write_in_text}
                                  contestId={question.id}
                                  index={answerIndex}
                                  isActive={!isReview}
                                  isReview={isReview}
                                  contest={question}
                                  url={url}
                              />
                          ))
                        : candidatesOrder
                              ?.map((id) => noCategoryCandidatesMap[id])
                              .map((answer, answerIndex) => (
                                  <Answer
                                      isInvalidWriteIns={isInvalidWriteIns}
                                      ballotStyle={ballotStyle}
                                      answer={answer}
                                      questionPlaintext={questionPlaintext}
                                      contestId={question.id}
                                      index={answerIndex}
                                      key={answerIndex}
                                      isActive={!isReview}
                                      isReview={isReview}
                                      isRadioSelection={isRadioSelection}
                                      contest={question}
                                      selectedChoicesSum={selectedChoicesSum}
                                      setSelectedChoicesSum={setSelectedChoicesSum}
                                      disableSelect={disableSelect}
                                      iconCheckboxPolicy={iconCheckboxPolicy}
                                      onResetBallotSelection={onResetBallotSelection}
                                      onSetBallotSelectionBlankVote={onSetBallotSelectionBlankVote}
                                      onSetBallotSelectionInvalidVote={
                                          onSetBallotSelectionInvalidVote
                                      }
                                      onSetBallotSelectionVoteChoice={
                                          onSetBallotSelectionVoteChoice
                                      }
                                      url={url}
                                  />
                              ))}
                </CandidatesSingleWrapper>
                {invalidBottomCandidates.map((answer, answerIndex) => (
                    <Answer
                        ballotStyle={ballotStyle}
                        answer={answer}
                        contestId={question.id}
                        index={answerIndex}
                        key={answerIndex}
                        isActive={!isReview}
                        isReview={isReview}
                        isInvalidVote={checkIsInvalidVote(answer)}
                        isExplicitBlankVote={checkIsExplicitBlankVote(answer)}
                        isInvalidWriteIns={false}
                        isRadioSelection={isRadioSelection}
                        contest={question}
                        selectedChoicesSum={selectedChoicesSum}
                        setSelectedChoicesSum={setSelectedChoicesSum}
                        disableSelect={disableSelect}
                        iconCheckboxPolicy={iconCheckboxPolicy}
                        onResetBallotSelection={onResetBallotSelection}
                        onSetBallotSelectionBlankVote={onSetBallotSelectionBlankVote}
                        onSetBallotSelectionInvalidVote={onSetBallotSelectionInvalidVote}
                        onSetBallotSelectionVoteChoice={onSetBallotSelectionVoteChoice}
                        url={url}
                    />
                ))}
            </CandidatesWrapper>
        </Box>
    )
}
