// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box} from "@mui/material"
import {theme, stringToHtml, shuffle, splitList, WarnBox} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"
import Typography from "@mui/material/Typography"
import {IQuestion} from "sequent-core"
import {Answer} from "../Answer/Answer"
import {AnswersList} from "../AnswersList/AnswersList"
import {
    checkPositionIsTop,
    checkShuffleAllOptions,
    checkShuffleCategories,
    checkShuffleCategoryList,
    getCheckableOptions,
} from "../../services/ElectionConfigService"
import {categorizeCandidates, getShuffledCategories} from "../../services/CategoryService"
import {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {provideBallotService} from "../../services/BallotService"
import {useAppSelector} from "../../store/hooks"
import {selectBallotSelectionByElectionId} from "../../store/ballotSelections/ballotSelectionsSlice"
import {useTranslation} from "react-i18next"
import {IBallotStyle as IElectionDTO} from "sequent-core"

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

interface IInvalidErrorsListProps {
    ballotStyle: IBallotStyle
    question: IQuestion
}

const InvalidErrorsList: React.FC<IInvalidErrorsListProps> = ({ballotStyle, question}) => {
    const {t} = useTranslation()
    const selectionState = useAppSelector(
        selectBallotSelectionByElectionId(ballotStyle.election_id)
    )
    const {interpretBallotSelection} = provideBallotService()

    const decodedSelection =
        selectionState && interpretBallotSelection(selectionState, ballotStyle.ballot_eml)
    const decodedContestSelection = decodedSelection?.find(
        (contest) => contest.contest_id === question.id
    )

    return (
        <>
            {decodedContestSelection?.invalid_errors.map((error, index) => (
                <WarnBox variant="warning" key={index}>
                    {t(
                        error.message || "",
                        error.message_map && Object.fromEntries(error.message_map)
                    )}
                </WarnBox>
            ))}
        </>
    )
}

export interface IQuestionProps {
    ballotStyle: IBallotStyle
    question: IQuestion
    questionIndex: number
    isReview: boolean
}

export const Question: React.FC<IQuestionProps> = ({
    ballotStyle,
    question,
    questionIndex,
    isReview,
}) => {
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
    categoriesMap = getShuffledCategories(
        categoriesMap,
        shuffleAllOptions,
        shuffleCategories,
        shuffleCategoryList
    )

    if (shuffleAllOptions) {
        noCategoryCandidates = shuffle(noCategoryCandidates)
    }

    return (
        <Box>
            <StyledTitle variant="h5">{question.title}</StyledTitle>
            {question.description ? (
                <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                    {stringToHtml(question.description)}
                </Typography>
            ) : null}
            <CandidatesWrapper>
                <InvalidErrorsList ballotStyle={ballotStyle} question={question} />
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
                {Object.entries(categoriesMap).map(([categoryName, category], categoryIndex) => (
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
                    />
                ))}
                {noCategoryCandidates.map((answer, answerIndex) => (
                    <Answer
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
                    />
                ))}
            </CandidatesWrapper>
        </Box>
    )
}
