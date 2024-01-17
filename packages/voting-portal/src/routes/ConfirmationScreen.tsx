// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Typography} from "@mui/material"
import React, {useState, useEffect} from "react"
import {useTranslation} from "react-i18next"
import {
    PageLimit,
    BreadCrumbSteps,
    Icon,
    IconButton,
    stringToHtml,
    theme,
    QRCode,
    Dialog,
} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"
import {faPrint, faCircleQuestion, faCheck} from "@fortawesome/free-solid-svg-icons"
import Button from "@mui/material/Button"
import {useNavigate, useParams} from "react-router-dom"
import Link from "@mui/material/Link"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {selectAuditableBallot} from "../store/auditableBallots/auditableBallotsSlice"
import {provideBallotService} from "../services/BallotService"
import {hasVotedAllElections} from "../store/castVotes/castVotesSlice"
import {TenantEventType} from ".."
import {useRootBackLink} from "../hooks/root-back-link"
import {resetBallotSelection} from "../store/ballotSelections/ballotSelectionsSlice"
import {selectBallotStyleByElectionId} from "../store/ballotStyles/ballotStylesSlice"

const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
`

const StyledButton = styled(Button)`
    display flex;
    padding: 5px;

    span {
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        padding: 5px;
    }
`

const ActionsContainer = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    gap: 2px;
`

const BallotIdContainer = styled(Box)`
    display: flex;
    flex-direction: row;
    gap: 30px;
    margin: 25px 0;
    align-items: center;
`

const BallotIdBorder = styled(Box)`
    background-color: ${({theme}) => theme.palette.green.light};
    color: ${({theme}) => theme.palette.customGrey.contrastText};
    padding: 10px 12px;
    display: flex;
    flex-direction: row;
    justify-content: left;
    align-items: center;
    gap: 10px;
    border-radius: 4px;
`

const BallotIdLink = styled(Link)`
    color: ${({theme}) => theme.palette.brandColor};
    text-decoration: none;
    font-weight: normal;
    overflow-wrap: anywhere;
    text-overflow: ellipsis;
    &:hover {
        text-decoration: underline;
    }
`

const QRContainer = styled(Box)`
    display: flex;
    justify-content: center;
    width: 100%;
    margin: 15px auto;
`

const ActionLink = styled(Link)`ç
    text-decoration: none;
    &:hover {
        text-decoration: none;
    }
`

interface ActionButtonsProps {
    electionId?: string
}

const ActionButtons: React.FC<ActionButtonsProps> = ({electionId}) => {
    const {t} = useTranslation()
    const {tenantId, eventId} = useParams<TenantEventType>()
    const castVotes = useAppSelector(hasVotedAllElections(String(electionId)))
    const triggerPrint = () => window.print()
    const navigate = useNavigate()
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionId)))
    const dispatch = useAppDispatch()

    const onClickToScreen = () => {
        navigate(`/tenant/${tenantId}/event/${eventId}/election-chooser`)
    }

    useEffect(() => {
        if (ballotStyle) {
            dispatch(
                resetBallotSelection({
                    ballotStyle,
                    force: true,
                })
            )
        }
    }, [ballotStyle, dispatch])

    return (
        <ActionsContainer>
            <StyledButton
                onClick={triggerPrint}
                variant="secondary"
                sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
            >
                <Icon icon={faPrint} size="sm" />
                <Box>{t("confirmationScreen.printButton")}</Box>
            </StyledButton>
            {castVotes ? (
                <ActionLink
                    href="https://sequentech.io"
                    sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
                >
                    <StyledButton sx={{width: {xs: "100%", sm: "200px"}}}>
                        <Box>{t("confirmationScreen.finishButton")}</Box>
                    </StyledButton>
                </ActionLink>
            ) : (
                <StyledButton
                    className="finish-button"
                    onClick={onClickToScreen}
                    sx={{width: {xs: "100%", sm: "200px"}}}
                >
                    <Box>{t("confirmationScreen.finishButton")}</Box>
                </StyledButton>
            )}
        </ActionsContainer>
    )
}

export const ConfirmationScreen: React.FC = () => {
    const {tenantId, eventId} = useParams<TenantEventType>()
    const {electionId} = useParams<{electionId?: string}>()
    const auditableBallot = useAppSelector(selectAuditableBallot(String(electionId)))
    const {hashBallot} = provideBallotService()
    const ballotId = (auditableBallot && hashBallot(auditableBallot)) || ""
    const {t} = useTranslation()
    const [openBallotIdHelp, setOpenBallotIdHelp] = useState(false)
    const [openConfirmationHelp, setOpenConfirmationHelp] = useState(false)

    const ballotTrackerUrl = `${window.location.protocol}//${window.location.host}/tenant/${tenantId}/event/${eventId}/election/${electionId}/ballot-locator/${ballotId}`

    const backLink = useRootBackLink()
    const navigate = useNavigate()

    useEffect(() => {
        if (!ballotId) {
            navigate(backLink)
        }
    })

    return (
        <PageLimit maxWidth="lg">
            <Box marginTop="24px">
                <BreadCrumbSteps
                    labels={[
                        "breadcrumbSteps.electionList",
                        "breadcrumbSteps.ballot",
                        "breadcrumbSteps.review",
                        "breadcrumbSteps.confirmation",
                    ]}
                    selected={3}
                />
            </Box>
            <StyledTitle variant="h4" fontSize="24px" fontWeight="bold" sx={{marginTop: "40px"}}>
                <Box>{t("confirmationScreen.title")}</Box>
                <IconButton
                    icon={faCircleQuestion}
                    sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                    fontSize="16px"
                    onClick={() => setOpenConfirmationHelp(true)}
                />
                <Dialog
                    handleClose={() => setOpenConfirmationHelp(false)}
                    open={openConfirmationHelp}
                    title={t("confirmationScreen.confirmationHelpDialog.title")}
                    ok={t("confirmationScreen.confirmationHelpDialog.ok")}
                    variant="info"
                >
                    {stringToHtml(t("confirmationScreen.confirmationHelpDialog.content"))}
                </Dialog>
            </StyledTitle>
            <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                {stringToHtml(t("confirmationScreen.description"))}
            </Typography>
            <BallotIdContainer>
                <Typography
                    variant="h5"
                    fontSize="18px"
                    fontWeight="bold"
                    sx={{display: {xs: "none", sm: "block"}}}
                >
                    {t("confirmationScreen.ballotId")}
                </Typography>
                <BallotIdBorder>
                    <IconButton
                        icon={faCheck}
                        sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                        fontSize="14px"
                        color={theme.palette.customGrey.contrastText}
                    />
                    <BallotIdLink
                        href={ballotTrackerUrl}
                        target="_blank"
                        sx={{display: {xs: "none", sm: "block"}}}
                    >
                        {ballotId}
                    </BallotIdLink>
                    <BallotIdLink
                        href={ballotTrackerUrl}
                        target="_blank"
                        sx={{display: {xs: "block", sm: "none"}}}
                    >
                        {t("ballotHash", {ballotId: ballotId})}
                    </BallotIdLink>
                    <IconButton
                        icon={faCircleQuestion}
                        sx={{
                            fontSize: "unset",
                            lineHeight: "unset",
                            paddingBottom: "2px",
                            marginLeft: "16px",
                        }}
                        fontSize="18px"
                        onClick={() => setOpenBallotIdHelp(true)}
                    />
                    <Dialog
                        handleClose={() => setOpenBallotIdHelp(false)}
                        open={openBallotIdHelp}
                        title={t("confirmationScreen.ballotIdHelpDialog.title")}
                        ok={t("confirmationScreen.ballotIdHelpDialog.ok")}
                        variant="info"
                    >
                        {stringToHtml(t("confirmationScreen.ballotIdHelpDialog.content"))}
                    </Dialog>
                </BallotIdBorder>
            </BallotIdContainer>
            <Typography variant="h5" fontSize="18px" fontWeight="bold">
                {t("confirmationScreen.verifyCastTitle")}
            </Typography>
            <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                {stringToHtml(t("confirmationScreen.verifyCastDescription"))}
            </Typography>
            <QRContainer>
                <QRCode value={ballotTrackerUrl} />
            </QRContainer>
            <ActionButtons electionId={electionId} />
        </PageLimit>
    )
}
