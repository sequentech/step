// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import Typography from "@mui/material/Typography"
import Box from "@mui/material/Box"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"
import {IBallotService} from "../services/BallotService"
import {IDecodedVoteContest, checkIsBlank} from "@sequentech/ui-core"
import {WarnBox, BlankAnswer} from "@sequentech/ui-essentials"
import {translate, IContest, EInvalidVotePolicy} from "@sequentech/ui-core"
import {keyBy} from "lodash"
import {checkIsInvalidVote, checkIsWriteIn} from "../services/ElectionConfigService"
import {VoteChoice} from "./VoteChoice"
import {CandidateChoice} from "./CandidateChoice"

const CandidatesWrapper = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin: 12px 0;
`

interface PlaintextVoteQuestionProps {
    questionPlaintext: IDecodedVoteContest
    question: IContest | null
    ballotService: IBallotService
}

export const PlaintextVoteQuestion: React.FC<PlaintextVoteQuestionProps> = ({
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

    console.log("aa questionPlaintext", questionPlaintext)

    return (
        <>
            <Typography variant="body2" fontWeight={"bold"}>
                {translate(question, "name", i18n.language) || ""}
            </Typography>
            {isBlank ? <BlankAnswer /> : null}
            {questionPlaintext.invalid_errors.map((error, index) => (
                <WarnBox variant="warning" key={index}>
                    {t(
                        error.message || "",
                        error.message_map && Object.fromEntries(error.message_map)
                    )}
                </WarnBox>
            ))}
            {questionPlaintext.is_explicit_invalid ? (
                <VoteChoice
                    text={explicitInvalidAnswer?.name || t("confirmationScreen.markedInvalid")}
                    points={null}
                    ordered={properties?.ordered || false}
                />
            ) : null}
            <CandidatesWrapper>
                {selectedAnswers.map((answer, index) => (
                    <CandidateChoice
                        key={index}
                        answer={answersById[answer.id]}
                        points={(showPoints && ballotService.getPoints(question, answer)) || null}
                        ordered={properties?.ordered || false}
                        isWriteIn={checkIsWriteIn(answersById[answer.id])}
                        writeInValue={answer.write_in_text}
                    />
                ))}
            </CandidatesWrapper>
        </>
    )
}
