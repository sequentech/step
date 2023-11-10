// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {Box} from "@mui/material"
import {theme, stringToHtml, shuffle, splitList, keyBy} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"
import Typography from "@mui/material/Typography"
import {IContest} from "sequent-core"
import {Answer} from "../Answer/Answer"
import {AnswersList} from "../AnswersList/AnswersList"
import {
    checkPositionIsTop,
    checkShuffleAllOptions,
    checkShuffleCategories,
    checkShuffleCategoryList,
    getCheckableOptions,
} from "../../services/ElectionConfigService"
import {
    CategoriesMap,
    categorizeCandidates,
    getShuffledCategories,
} from "../../services/CategoryService"
import {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {InvalidErrorsList} from "../InvalidErrorsList/InvalidErrorsList"

const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
`

const CandidatesWrapper = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin: 12px 0;
`

export interface IQuestionProps {
    ballotStyle: IBallotStyle
    question: IContest
    questionIndex: number
    isReview: boolean
    setDisableNext?: (value: boolean) => void
}

export const Question: React.FC<IQuestionProps> = ({
    ballotStyle,
    question,
    questionIndex,
    isReview,
    setDisableNext,
}) => {
    let [candidatesOrder, setCandidatesOrder] = useState<Array<string> | null>(null)
    let [categoriesMapOrder, setCategoriesMapOrder] = useState<CategoriesMap | null>(null)
    let [isInvalidWriteIns, setIsInvalidWriteIns] = useState(false)
    let {invalidCandidates, noCategoryCandidates, categoriesMap} = categorizeCandidates(question)
    const {checkableLists, checkableCandidates} = getCheckableOptions(question)
    let [invalidBottomCandidates, invalidTopCandidates] = splitList(
        invalidCandidates,
        checkPositionIsTop
    )

    // do the shuffling
    const shuffleAllOptions = checkShuffleAllOptions(question)
    const shuffleCategories = checkShuffleCategories(question)
    const shuffleCategoryList = checkShuffleCategoryList(question)
    if (null === categoriesMapOrder) {
        setCategoriesMapOrder(
            getShuffledCategories(
                categoriesMap,
                shuffleAllOptions,
                shuffleCategories,
                shuffleCategoryList
            )
        )
    }

    if (null === candidatesOrder) {
        if (shuffleAllOptions) {
            setCandidatesOrder(shuffle(noCategoryCandidates.map((c) => c.id)))
        } else {
            setCandidatesOrder(noCategoryCandidates.map((c) => c.id).sort())
        }
    }

    if (shuffleAllOptions && null === candidatesOrder) {
        setCandidatesOrder(shuffle(noCategoryCandidates.map((c) => c.id)))
    }
    const noCategoryCandidatesMap = keyBy(noCategoryCandidates, "id")

    const onSetIsInvalidWriteIns = (value: boolean) => {
        setIsInvalidWriteIns(value)
        setDisableNext?.(value)
    }

    return (
        <Box>
            <StyledTitle variant="h5">{question.name || ""}</StyledTitle>
            {question.description ? (
                <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                    {stringToHtml(question.description)}
                </Typography>
            ) : null}
            <CandidatesWrapper>
                <InvalidErrorsList
                    ballotStyle={ballotStyle}
                    question={question}
                    isInvalidWriteIns={isInvalidWriteIns}
                    setIsInvalidWriteIns={onSetIsInvalidWriteIns}
                />
                {invalidTopCandidates.map((answer, answerIndex) => (
                    <Answer
                        ballotStyle={ballotStyle}
                        answer={answer}
                        questionIndex={questionIndex}
                        key={answerIndex}
                        isActive={!isReview}
                        isReview={isReview}
                        isInvalidVote={true}
                    />
                ))}
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
                                questionIndex={questionIndex}
                                isReview={isReview}
                                isInvalidWriteIns={isInvalidWriteIns}
                            />
                        )
                    )}
                {candidatesOrder
                    ?.map((id) => noCategoryCandidatesMap[id])
                    .map((answer, answerIndex) => (
                        <Answer
                            isInvalidWriteIns={isInvalidWriteIns}
                            ballotStyle={ballotStyle}
                            answer={answer}
                            questionIndex={questionIndex}
                            key={answerIndex}
                            isActive={!isReview}
                            isReview={isReview}
                        />
                    ))}
                {invalidBottomCandidates.map((answer, answerIndex) => (
                    <Answer
                        ballotStyle={ballotStyle}
                        answer={answer}
                        questionIndex={questionIndex}
                        key={answerIndex}
                        isActive={!isReview}
                        isReview={isReview}
                        isInvalidVote={true}
                        isInvalidWriteIns={false}
                    />
                ))}
            </CandidatesWrapper>
        </Box>
    )
}
