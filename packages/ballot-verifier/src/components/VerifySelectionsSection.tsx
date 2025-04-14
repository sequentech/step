// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import Typography from "@mui/material/Typography"
import Paper from "@mui/material/Paper"
import Box from "@mui/material/Box"
import {Link as RouterLink} from "react-router-dom"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"
import Skeleton from "@mui/material/Skeleton"
import {IBallotService, IConfirmationBallot} from "../services/BallotService"
import Button from "@mui/material/Button"
import {faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import {IconButton, Dialog, theme} from "@sequentech/ui-essentials"
import {keyBy} from "lodash"
import {PlaintextVoteQuestion} from "./PlaintextVoteQuestion"

const HorizontalWrap = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 16px;
    margin-bottom: 12px;
`

interface VerifySelectionsSectionProps {
    isLoading: boolean
    confirmationBallot: IConfirmationBallot | null
    ballotService: IBallotService
}

export const VerifySelectionsSection: React.FC<VerifySelectionsSectionProps> = ({
    isLoading,
    confirmationBallot,
    ballotService,
}) => {
    const {t} = useTranslation()
    const [verifySelectionsHelp, setVerifySelectionsHelp] = useState(false)
    const plaintextVoteQuestions = confirmationBallot?.decoded_questions || []
    const questionsMap = keyBy(confirmationBallot?.election_config.contests || [], "id")

    return (
        <>
            <HorizontalWrap marginTop="26px">
                <Typography variant="h5">
                    {t("confirmationScreen.verifySelectionsTitle")}
                </Typography>
                <IconButton
                    icon={faCircleQuestion}
                    sx={{
                        fontSize: "unset",
                        lineHeight: "unset",
                        paddingBottom: "2px",
                    }}
                    fontSize="16px"
                    onClick={() => setVerifySelectionsHelp(true)}
                />
                <Dialog
                    handleClose={() => setVerifySelectionsHelp(false)}
                    open={verifySelectionsHelp}
                    title={t("confirmationScreen.verifySelectionsHelpDialog.title")}
                    ok={t("confirmationScreen.verifySelectionsHelpDialog.ok")}
                    variant="info"
                >
                    <p>{t("confirmationScreen.verifySelectionsHelpDialog.content")}</p>
                </Dialog>
            </HorizontalWrap>
            <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                {t("confirmationScreen.verifySelectionsDescription")}
            </Typography>
            {isLoading ? (
                <>
                    <Skeleton variant="text" />
                    <Skeleton variant="text" />
                </>
            ) : (
                <>
                    <Typography variant="h5" textAlign="left">
                        {confirmationBallot?.election_config.description}
                    </Typography>
                    <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                        {confirmationBallot?.election_config.description}
                    </Typography>
                </>
            )}
            {isLoading ? (
                <>
                    <Skeleton variant="text" />
                    <Skeleton variant="text" />
                    <Skeleton variant="text" width={200} />
                    <Skeleton variant="text" width={50} />
                </>
            ) : (
                <>
                    {plaintextVoteQuestions.map((voteQuestion) => (
                        <PlaintextVoteQuestion
                            questionPlaintext={voteQuestion}
                            question={questionsMap[voteQuestion.contest_id] ?? null}
                            ballotService={ballotService}
                            key={voteQuestion.contest_id}
                        />
                    ))}
                </>
            )}
        </>
    )
}
