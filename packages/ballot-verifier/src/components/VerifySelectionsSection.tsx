// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import Typography from "@mui/material/Typography"
import Box from "@mui/material/Box"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"
import Skeleton from "@mui/material/Skeleton"
import {IConfirmationBallot} from "../services/BallotService"
import {faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import {IconButton, Dialog, theme, ContestDisplay} from "@sequentech/ui-essentials"
import {keyBy} from "lodash"

import {IBallotStyle as IElectionDTO} from "@sequentech/ui-core"

export interface IBallotStyle {
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

const HorizontalWrap = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 16px;
    margin-bottom: 12px;
`

interface VerifySelectionsSectionProps {
    ballotStyle: IBallotStyle
    isLoading: boolean
    confirmationBallot: IConfirmationBallot | null
}

export const VerifySelectionsSection: React.FC<VerifySelectionsSectionProps> = ({
    ballotStyle,
    isLoading,
    confirmationBallot,
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
            {!ballotStyle && isLoading ? (
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
            {!ballotStyle && isLoading ? (
                <>
                    <Skeleton variant="text" />
                    <Skeleton variant="text" />
                    <Skeleton variant="text" width={200} />
                    <Skeleton variant="text" width={50} />
                </>
            ) : (
                <>
                    {ballotStyle &&
                        plaintextVoteQuestions.map((voteQuestion) => (
                            <ContestDisplay
                                ballotStyle={ballotStyle}
                                question={questionsMap[voteQuestion.contest_id] ?? null}
                                questionPlaintext={voteQuestion}
                                isReview={true}
                                isVotedState={true}
                                key={voteQuestion.contest_id}
                                errorSelectionState={[]}
                            />
                        ))}
                </>
            )}
        </>
    )
}
