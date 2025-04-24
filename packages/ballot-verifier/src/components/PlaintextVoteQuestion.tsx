// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import Typography from "@mui/material/Typography"
import Box from "@mui/material/Box"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"
import emotionStyled from "@emotion/styled"
import {keyBy} from "lodash"

import {IBallotStyle, IDecodedVoteContest, checkIsBlank} from "@sequentech/ui-core"
import {translate, IContest, EInvalidVotePolicy} from "@sequentech/ui-core"
import {WarnBox, BlankAnswer, theme} from "@sequentech/ui-essentials"

import {IBallotService} from "../services/BallotService"
import {checkIsInvalidVote, checkIsWriteIn} from "../services/ElectionConfigService"

import {VoteChoice} from "./VoteChoice"
import {CandidateChoice} from "./CandidateChoice"

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

interface PlaintextVoteQuestionProps {
    ballotStyle: IBallotStyle | undefined
    questionPlaintext: IDecodedVoteContest
    question: IContest | null
    ballotService: IBallotService
}

export const PlaintextVoteQuestion: React.FC<PlaintextVoteQuestionProps> = ({
    ballotStyle,
    questionPlaintext,
    question,
    ballotService,
}) => {
    const {t, i18n} = useTranslation()
    const selectedAnswers = questionPlaintext.choices.filter((a) => a.selected > -1)
    if (!question) {
        return (
            <>
                {t("confirmationScreen.contestNotFound", {contestId: questionPlaintext.contest_id})}
            </>
        )
    }
    const explicitInvalidAnswer =
        (questionPlaintext.is_explicit_invalid &&
            question.presentation?.invalid_vote_policy !== EInvalidVotePolicy.NOT_ALLOWED &&
            question.candidates.find((answer) => checkIsInvalidVote(answer))) ||
        null
    const answersById = keyBy(question.candidates, (a) => a.id)
    const properties = ballotService.getLayoutProperties(question)
    const showPoints = !!question.presentation?.show_points
    const isBlank = checkIsBlank(questionPlaintext)
    const columnCount = question.presentation?.columns ?? 1

    console.log("aa questionPlaintext", questionPlaintext)
    console.log("aa selectedAnswers", selectedAnswers)
    console.log("aa question", question)

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
            {questionPlaintext.invalid_errors.map((error, index) => (
                <WarnBox variant="warning" key={index}>
                    {t(
                        error.message || "",
                        error.message_map && Object.fromEntries(error.message_map)
                    )}
                </WarnBox>
            ))}
            {isBlank ? <BlankAnswer /> : null}
            {questionPlaintext.is_explicit_invalid ? (
                <VoteChoice
                    text={explicitInvalidAnswer?.name || t("confirmationScreen.markedInvalid")}
                    points={null}
                    ordered={properties?.ordered || false}
                />
            ) : null}
            <CandidatesSingleWrapper
                className="candidates-singles-container"
                columnCount={columnCount}
            >
                {selectedAnswers.map((answer, index) => (
                    <CandidateChoice
                        ballotStyle={ballotStyle}
                        key={index}
                        answer={answersById[answer.id]}
                        points={(showPoints && ballotService.getPoints(question, answer)) || null}
                        ordered={properties?.ordered || false}
                        isWriteIn={checkIsWriteIn(answersById[answer.id])}
                        writeInValue={answer.write_in_text}
                    />
                ))}
            </CandidatesSingleWrapper>
        </Box>
    )
}
