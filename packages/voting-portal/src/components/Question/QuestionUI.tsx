// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box} from "@mui/material"
import {
    IContest,
    ECandidatesIconCheckboxPolicy,
    BallotSelection,
    IDecodedVoteContest,
    ICandidate,
} from "@sequentech/ui-core"
import {CategoriesMap, ICategory} from "../../services/CategoryService"
import {theme, BlankAnswer} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"
import emotionStyled from "@emotion/styled"
import Typography from "@mui/material/Typography"
import {Answer} from "../Answer/Answer"
import {AnswersList} from "../AnswersList/AnswersList"
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

export interface IQuestionUIProps {
    title: string
    description?: string | React.ReactNode
    isReview: boolean
    isBlank: boolean
    columnCount: number
    hasWriteIns: boolean
    isInvalidWriteIns: boolean
    setIsInvalidWriteIns: (value: boolean) => void
    ballotStyle: IBallotStyle
    question: IContest
    candidatesOrder: Array<string> | null
    noCategoryCandidatesMap: Record<string, ICandidate>
    categoriesMapOrder: CategoriesMap | null
    checkableLists: boolean
    checkableCandidates: boolean
    selectedChoicesSum: number
    setSelectedChoicesSum: (num: number) => void
    disableSelect: boolean
    isRadioSelection: boolean
    iconCheckboxPolicy: ECandidatesIconCheckboxPolicy
    setDecodedContests: (input: IDecodedVoteContest) => void
    errorSelectionState: BallotSelection
}

export const QuestionUI: React.FC<IQuestionUIProps> = ({
    title,
    description,
    isReview,
    isBlank,
    columnCount,
    hasWriteIns,
    isInvalidWriteIns,
    setIsInvalidWriteIns,
    ballotStyle,
    question,
    candidatesOrder,
    noCategoryCandidatesMap,
    categoriesMapOrder,
    checkableLists,
    checkableCandidates,
    selectedChoicesSum,
    setSelectedChoicesSum,
    disableSelect,
    isRadioSelection,
    iconCheckboxPolicy,
    setDecodedContests,
    errorSelectionState,
}) => {
    return (
        <Box>
            <StyledTitle
                className="contest-title"
                variant="h5"
                data-min={question.min_votes}
                data-max={question.max_votes}
            >
                {title}
            </StyledTitle>
            {description ? (
                <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                    {description}
                </Typography>
            ) : null}
            <InvalidErrorsList
                ballotStyle={ballotStyle}
                question={question}
                hasWriteIns={hasWriteIns}
                isInvalidWriteIns={isInvalidWriteIns}
                setIsInvalidWriteIns={setIsInvalidWriteIns}
                setDecodedContests={setDecodedContests}
                isReview={isReview}
                errorSelectionState={errorSelectionState}
            />
            {isBlank ? <BlankAnswer /> : null}
            <CandidatesWrapper className="candidates-container">
                {categoriesMapOrder && Object.keys(categoriesMapOrder).length > 0 && (
                    <CandidateListsWrapper className="candidates-lists-container">
                        {Object.entries(categoriesMapOrder).map(
                            ([categoryName, category], categoryIndex) => (
                                <AnswersList
                                    key={categoryIndex}
                                    title={categoryName}
                                    isActive={true}
                                    checkableLists={checkableLists}
                                    checkableCandidates={checkableCandidates}
                                    category={category as ICategory}
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
                )}
                {candidatesOrder && candidatesOrder.length > 0 && (
                    <CandidatesSingleWrapper
                        className="candidates-singles-container"
                        columnCount={columnCount}
                    >
                        {candidatesOrder
                            .map((id) => noCategoryCandidatesMap[id])
                            .map((answer, answerIndex) => (
                                <Answer
                                    isInvalidWriteIns={isInvalidWriteIns}
                                    ballotStyle={ballotStyle}
                                    answer={answer}
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
                                />
                            ))}
                    </CandidatesSingleWrapper>
                )}
            </CandidatesWrapper>
        </Box>
    )
}
