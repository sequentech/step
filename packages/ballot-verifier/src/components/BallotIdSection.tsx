// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import Typography from "@mui/material/Typography"
import Paper from "@mui/material/Paper"
import Box from "@mui/material/Box"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"
import {IConfirmationBallot} from "../services/BallotService"
import {faCircleQuestion, faTimesCircle} from "@fortawesome/free-solid-svg-icons"
import {IconButton, Dialog, theme} from "@sequentech/ui-essentials"
import {ICandidate} from "@sequentech/ui-core"
import {BallotIdContainer} from "./BallotIdContainer"

const HorizontalWrap = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 16px;
    margin-bottom: 12px;
`

const OneLine = styled(Paper)`
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
`

enum VariantType {
    Info = "info",
    Error = "error",
}

interface BallotIdSectionProps {
    confirmationBallot: IConfirmationBallot | null
    ballotId: string
}

export const isMatchingBallotIds = (
    confirmationBallotId: String | undefined,
    ballotId: String
): boolean => {
    return confirmationBallotId === ballotId
}

const ballotMatchVariantType = (
    confirmationBallotId: string | undefined,
    ballotId: string
): VariantType => {
    return isMatchingBallotIds(confirmationBallotId, ballotId)
        ? VariantType.Info
        : VariantType.Error
}

export const BallotIdSection: React.FC<BallotIdSectionProps> = ({confirmationBallot, ballotId}) => {
    const {t} = useTranslation()
    const [decodedBallotIdHelp, setDecodedBallotIdHelp] = useState(false)
    const [userBallotIdHelp, setUserBallotIdHelp] = useState(false)

    return (
        <>
            <Typography variant="h5">{t("confirmationScreen.ballotIdTitle")}</Typography>
            <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                {t("confirmationScreen.ballotIdDescription")}
            </Typography>
            <HorizontalWrap>
                <Typography variant="h5" fontSize="16px" width="106px">
                    {t("confirmationScreen.decodedBallotId")}
                </Typography>
                <BallotIdContainer variant={VariantType.Info}>
                    <OneLine variant="info">{confirmationBallot?.ballot_hash}</OneLine>
                    <IconButton
                        icon={faCircleQuestion}
                        sx={{
                            fontSize: "unset",
                            lineHeight: "unset",
                            paddingBottom: "2px",
                            color: theme.palette.black,
                            marginLeft: "10px",
                        }}
                        fontSize="16px"
                        onClick={() => setDecodedBallotIdHelp(true)}
                    />
                    <Dialog
                        handleClose={() => setDecodedBallotIdHelp(false)}
                        open={decodedBallotIdHelp}
                        title={t("confirmationScreen.decodedBallotIdHelpDialog.title")}
                        ok={t("confirmationScreen.decodedBallotIdHelpDialog.ok")}
                        variant="info"
                    >
                        <p>{t("confirmationScreen.decodedBallotIdHelpDialog.content")}</p>
                    </Dialog>
                </BallotIdContainer>
            </HorizontalWrap>
            <HorizontalWrap>
                <Typography variant="h5" fontSize="16px" width="106px">
                    {t("confirmationScreen.yourBallotId")}
                </Typography>
                <Box sx={{overflow: "auto"}}>
                    <BallotIdContainer
                        variant={ballotMatchVariantType(confirmationBallot?.ballot_hash, ballotId)}
                        sx={{
                            marginTop: isMatchingBallotIds(
                                confirmationBallot?.ballot_hash,
                                ballotId
                            )
                                ? undefined
                                : "14px",
                        }}
                    >
                        {isMatchingBallotIds(confirmationBallot?.ballot_hash, ballotId) ? null : (
                            <IconButton
                                icon={faTimesCircle}
                                sx={{
                                    fontSize: "unset",
                                    lineHeight: "unset",
                                    paddingBottom: "2px",
                                    marginRight: "10px",
                                }}
                                fontSize="16px"
                            />
                        )}
                        <OneLine
                            variant={ballotMatchVariantType(
                                confirmationBallot?.ballot_hash,
                                ballotId
                            )}
                        >
                            {ballotId}
                        </OneLine>
                        <IconButton
                            icon={faCircleQuestion}
                            sx={{
                                fontSize: "unset",
                                lineHeight: "unset",
                                paddingBottom: "2px",
                                color: theme.palette.black,
                                marginLeft: "10px",
                            }}
                            fontSize="16px"
                            onClick={() => setUserBallotIdHelp(true)}
                        />
                        <Dialog
                            handleClose={() => setUserBallotIdHelp(false)}
                            open={userBallotIdHelp}
                            title={t("confirmationScreen.userBallotIdHelpDialog.title")}
                            ok={t("confirmationScreen.userBallotIdHelpDialog.ok")}
                            variant="info"
                        >
                            <p>{t("confirmationScreen.userBallotIdHelpDialog.content")}</p>
                        </Dialog>
                    </BallotIdContainer>
                    {isMatchingBallotIds(confirmationBallot?.ballot_hash, ballotId) ? null : (
                        <Typography fontSize="12px" color={theme.palette.red.dark} marginTop="2px">
                            {t("confirmationScreen.ballotIdError")}
                        </Typography>
                    )}
                </Box>
            </HorizontalWrap>
        </>
    )
}
