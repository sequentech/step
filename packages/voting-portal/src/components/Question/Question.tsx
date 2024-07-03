// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {Box} from "@mui/material"
import {
    stringToHtml,
    splitList,
    keyBy,
    translate,
    IContest,
    CandidatesOrder,
} from "@sequentech/ui-core"
import {theme, BlankAnswer} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"
import Typography from "@mui/material/Typography"
import {Answer} from "../Answer/Answer"
import {AnswersList} from "../AnswersList/AnswersList"
import {
    checkIsRadioSelection,
    checkPositionIsTop,
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
import {useTranslation} from "react-i18next"
import {IDecodedVoteContest, IInvalidPlaintextError} from "sequent-core"
import {useAppSelector} from "../../store/hooks"
import {selectBallotSelectionQuestion} from "../../store/ballotSelections/ballotSelectionsSlice"
import {checkIsBlank} from "../../services/BallotService"
import {sortCandidatesInContest} from "../../services/Core"

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

const CandidatesSingleWrapper = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin: 12px 0;
`

export interface IQuestionProps {
    ballotStyle: IBallotStyle
    question: IContest
    isReview: boolean
    setDisableNext?: (value: boolean) => void
    setDecodedContests: (input: IDecodedVoteContest) => void
}

export const Question: React.FC<IQuestionProps> = ({
    ballotStyle,
    question,
    isReview,
    setDisableNext,
    setDecodedContests,
}) => {
    const {i18n} = useTranslation()

    let [candidatesOrder, setCandidatesOrder] = useState<Array<string> | null>(null)
    let [categoriesMapOrder, setCategoriesMapOrder] = useState<CategoriesMap | null>(null)
    let [isInvalidWriteIns, setIsInvalidWriteIns] = useState(false)
    let {invalidCandidates, noCategoryCandidates, categoriesMap} = categorizeCandidates(question)
    const contestState = useAppSelector(
        selectBallotSelectionQuestion(ballotStyle.election_id, question.id)
    )
    const {checkableLists, checkableCandidates} = getCheckableOptions(question)
    let [invalidBottomCandidates, invalidTopCandidates] = splitList(
        invalidCandidates,
        checkPositionIsTop
    )

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
        <Box>
            <StyledTitle className="contest-title" variant="h5">
                {translate(question, "name", i18n.language) || ""}
            </StyledTitle>
            {question.description ? (
                <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                    {stringToHtml(translate(question, "description", i18n.language) || "")}
                </Typography>
            ) : null}
            <InvalidErrorsList
                ballotStyle={ballotStyle}
                question={question}
                isInvalidWriteIns={isInvalidWriteIns}
                setIsInvalidWriteIns={onSetIsInvalidWriteIns}
                setDecodedContests={setDecodedContests}
                isReview={isReview}
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
                        isInvalidVote={true}
                        isRadioSelection={isRadioSelection}
                        contest={question}
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
                                />
                            )
                        )}
                </CandidateListsWrapper>
                <CandidatesSingleWrapper className="candidates-singles-container">
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
                                isReview={isReview}
                                isRadioSelection={isRadioSelection}
                                contest={question}
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
                        isInvalidVote={true}
                        isInvalidWriteIns={false}
                        isRadioSelection={isRadioSelection}
                        contest={question}
                    />
                ))}
            </CandidatesWrapper>
        </Box>
    )
}
