// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Typography} from "@mui/material"
import React, {useState, useEffect, useContext, useRef} from "react"
import {useTranslation} from "react-i18next"
import {
    PageLimit,
    Icon,
    IconButton,
    stringToHtml,
    theme,
    QRCode,
    Dialog,
    IElectionEventPresentation,
} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"
import {faPrint, faCircleQuestion, faCheck} from "@fortawesome/free-solid-svg-icons"
import Button from "@mui/material/Button"

import {useLocation, useNavigate, useParams} from "react-router-dom"
import Link from "@mui/material/Link"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {selectAuditableBallot} from "../store/auditableBallots/auditableBallotsSlice"
import {canVoteSomeElection} from "../store/castVotes/castVotesSlice"
import {selectElectionEventById} from "../store/electionEvents/electionEventsSlice"
import {TenantEventType} from ".."
import {useRootBackLink} from "../hooks/root-back-link"
import {clearBallot} from "../store/ballotSelections/ballotSelectionsSlice"
import {
    selectBallotStyleByElectionId,
    selectFirstBallotStyle,
} from "../store/ballotStyles/ballotStylesSlice"
import {AuthContext} from "../providers/AuthContextProvider"
import {useLazyQuery, useMutation} from "@apollo/client"
import {CREATE_VOTE_RECEIPT} from "../queries/CreateVoteReceipt"
import {GET_DOCUMENT} from "../queries/GetDocument"
import {useGetPublicDocumentUrl} from "../hooks/public-document-url"
import Stepper from "../components/Stepper"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {provideBallotService} from "../services/BallotService"
import {VotingPortalError, VotingPortalErrorType} from "../services/VotingPortalError"

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

const ActionLink = styled(Link)`
    text-decoration: none;
    &:hover {
        text-decoration: none;
    }
`

interface ActionButtonsProps {
    electionId?: string
    ballotTrackerUrl?: string
    ballotId: string
}

const ActionButtons: React.FC<ActionButtonsProps> = ({ballotTrackerUrl, electionId, ballotId}) => {
    const {logout} = useContext(AuthContext)
    const {t} = useTranslation()
    const {tenantId, eventId} = useParams<TenantEventType>()
    const canVote = useAppSelector(canVoteSomeElection())
    const navigate = useNavigate()
    const location = useLocation()
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionId)))
    const dispatch = useAppDispatch()
    const auditableBallot = useAppSelector(selectAuditableBallot(String(electionId)))
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const [createVoteReceipt] = useMutation(CREATE_VOTE_RECEIPT)
    const [getDocument, {data: documentData}] = useLazyQuery(GET_DOCUMENT)
    const [polling, setPolling] = useState<NodeJS.Timer | null>(null)
    const [documentId, setDocumentId] = useState<string | null>(null)
    const [documentOpened, setDocumentOpened] = useState<boolean>(false)
    const [documentUrl, setDocumentUrl] = useState<string | null>(null)
    const documentUrlRef = useRef(documentUrl)
    const {getDocumentUrl} = useGetPublicDocumentUrl()
    const {globalSettings} = useContext(SettingsContext)
    const [errorDialog, setErrorDialog] = useState<boolean>(false)
    const [openPrintDemoModal, setOpenPrintDemoModal] = useState<boolean>(false)
    const oneBallotStyle = useAppSelector(selectFirstBallotStyle)
    const isDemo = oneBallotStyle?.ballot_eml.public_key?.is_demo

    let presentation = electionEvent?.presentation as IElectionEventPresentation | undefined

    const onClickToScreen = () => {
        navigate(`/tenant/${tenantId}/event/${eventId}/election-chooser${location.search}`)
    }

    const onClickRedirect = () => {
        logout(presentation?.redirect_finish_url ?? undefined)
    }

    useEffect(() => {
        if (ballotStyle) {
            dispatch(clearBallot())
        }
    }, [ballotStyle, dispatch])

    async function printVoteReceipt() {
        if (isDemo) {
            setOpenPrintDemoModal(true)
            return
        }

        if (documentUrl) {
            return window.open(documentUrl, "_blank")
        }

        const res = await createVoteReceipt({
            variables: {
                ballot_id: ballotId,
                ballot_tracker_url: ballotTrackerUrl,
                election_event_id: eventId,
                tenant_id: tenantId,
                election_id: electionId,
            },
        })

        let docId = res.data?.create_vote_receipt?.id

        if (docId) {
            setDocumentId(docId)
            startPolling(docId)
            setDocumentOpened(false)
        }
    }

    function fetchData(documentId: string) {
        getDocument({
            variables: {
                id: documentId,
                tenantId,
            },
            fetchPolicy: "network-only",
        })
    }

    function startPolling(documentId: string) {
        if (!polling) {
            fetchData(documentId)

            const intervalId = setInterval(() => {
                fetchData(documentId)
            }, 1000)

            setPolling(intervalId)

            setTimeout(() => {
                setPolling(null)
                if (!documentUrlRef.current) {
                    setErrorDialog(true)
                }
            }, globalSettings.POLLING_DURATION_TIMEOUT)
        }
    }

    useEffect(() => {
        documentUrlRef.current = documentUrl
    }, [documentUrl])

    useEffect(() => {
        function stopPolling() {
            if (polling) {
                clearInterval(polling)
                setPolling(null)
            }
        }

        if (documentData?.sequent_backend_document?.length > 0) {
            stopPolling()

            if (!documentOpened) {
                const newDocumentUrl = getDocumentUrl(
                    documentId!,
                    documentData?.sequent_backend_document[0]?.name
                )

                setDocumentUrl(newDocumentUrl)
                setDocumentOpened(true)

                setTimeout(() => {
                    // We use a setTimeout as a work around due to this issue in React:
                    // https://stackoverflow.com/questions/76944918/should-not-already-be-working-on-window-open-in-simple-react-app
                    // https://github.com/facebook/react/issues/17355
                    window.open(newDocumentUrl, "_blank")
                }, 0)
            }
        }
    }, [eventId, documentUrl, documentOpened, polling, documentData, documentId, getDocumentUrl])

    useEffect(() => {
        return () => {
            if (polling) {
                clearInterval(polling)
            }
        }
    }, [polling])

    return (
        <>
            <ActionsContainer>
                <StyledButton
                    onClick={printVoteReceipt}
                    disabled={!!polling}
                    variant="secondary"
                    sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
                >
                    <Icon icon={faPrint} size="sm" />
                    <Box>{t("confirmationScreen.printButton")}</Box>
                </StyledButton>
                {!canVote ? (
                    <ActionLink sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}>
                        <StyledButton
                            onClick={onClickRedirect}
                            className="finish-button"
                            sx={{width: {xs: "100%", sm: "200px"}}}
                        >
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

            <Dialog
                handleClose={() => setOpenPrintDemoModal(false)}
                open={openPrintDemoModal}
                title={t("confirmationScreen.demoPrintDialog.title")}
                ok={t("confirmationScreen.demoPrintDialog.ok")}
                variant="info"
            >
                {stringToHtml(t("confirmationScreen.demoPrintDialog.content"))}
            </Dialog>
            <Dialog
                handleClose={() => setErrorDialog(false)}
                open={errorDialog}
                title={t("confirmationScreen.errorDialogPrintVoteReceipt.title")}
                ok={t("confirmationScreen.errorDialogPrintVoteReceipt.ok")}
                variant="warning"
            >
                {stringToHtml(t("confirmationScreen.errorDialogPrintVoteReceipt.content"))}
            </Dialog>
        </>
    )
}

const ConfirmationScreen: React.FC = () => {
    const {tenantId, eventId} = useParams<TenantEventType>()
    const {electionId} = useParams<{electionId?: string}>()
    const auditableBallot = useAppSelector(selectAuditableBallot(String(electionId)))
    const {t} = useTranslation()
    const [openBallotIdHelp, setOpenBallotIdHelp] = useState(false)
    const [openConfirmationHelp, setOpenConfirmationHelp] = useState(false)
    const [openDemoBallotUrlHelp, setDemoBallotUrlHelp] = useState(false)
    const {hashBallot} = provideBallotService()
    const ballotId = (auditableBallot && hashBallot(auditableBallot)) || ""

    const ballotTrackerUrl = `${window.location.protocol}//${window.location.host}/tenant/${tenantId}/event/${eventId}/election/${electionId}/ballot-locator/${ballotId}`

    const backLink = useRootBackLink()
    const navigate = useNavigate()
    const [demoBallotIdHelp, setDemoBallotIdHelp] = useState<boolean>(false)
    const oneBallotStyle = useAppSelector(selectFirstBallotStyle)
    const isDemo = oneBallotStyle?.ballot_eml.public_key?.is_demo

    if (ballotId && auditableBallot?.ballot_hash && ballotId !== auditableBallot.ballot_hash) {
        console.log(
            `ballotId: ${ballotId}\n auditable Ballot Hash: ${auditableBallot?.ballot_hash}`
        )
        throw new VotingPortalError(VotingPortalErrorType.INCONSISTENT_HASH)
    }

    useEffect(() => {
        if (!ballotId) {
            navigate(backLink)
        }
    })

    const handleBallotIdLinkClick = (event: React.MouseEvent<HTMLAnchorElement, MouseEvent>) => {
        if (isDemo) {
            event.preventDefault()
            setDemoBallotUrlHelp(true)
        }
    }

    return (
        <PageLimit maxWidth="lg" className="confirmation-screen screen">
            <Box marginTop="24px">
                <Stepper selected={3} />
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
                        href={!isDemo ? ballotTrackerUrl : undefined}
                        target={!isDemo ? "_blank" : undefined}
                        sx={{display: {xs: "none", sm: "block"}}}
                        onClick={handleBallotIdLinkClick}
                    >
                        {ballotId}
                    </BallotIdLink>
                    <BallotIdLink
                        href={!isDemo ? ballotTrackerUrl : undefined}
                        target={!isDemo ? "_blank" : undefined}
                        sx={{display: {xs: "block", sm: "none"}}}
                        onClick={handleBallotIdLinkClick}
                    >
                        {t("ballotHash", {ballotId: ballotId})}
                    </BallotIdLink>
                    <IconButton
                        icon={faCircleQuestion}
                        sx={{
                            fontSize: "unset",
                            lineHeight: "unset",
                            marginLeft: "16px",
                        }}
                        fontSize="18px"
                        onClick={() =>
                            isDemo ? setDemoBallotIdHelp(true) : setOpenBallotIdHelp(true)
                        }
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
                    <Dialog
                        handleClose={() => setDemoBallotUrlHelp(false)}
                        open={openDemoBallotUrlHelp}
                        title={t("confirmationScreen.demoBallotUrlDialog.title")}
                        ok={t("confirmationScreen.demoBallotUrlDialog.ok")}
                        variant="info"
                    >
                        {stringToHtml(t("confirmationScreen.demoBallotUrlDialog.content"))}
                    </Dialog>
                    <Dialog
                        handleClose={() => setDemoBallotIdHelp(false)}
                        open={demoBallotIdHelp}
                        title={t("confirmationScreen.ballotIdDemoHelpDialog.title")}
                        ok={t("confirmationScreen.ballotIdDemoHelpDialog.ok")}
                        variant="info"
                    >
                        {stringToHtml(t("confirmationScreen.ballotIdDemoHelpDialog.content"))}
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
                <QRCode value={isDemo ? t("confirmationScreen.demoQRText") : ballotTrackerUrl} />
            </QRContainer>
            <ActionButtons
                ballotTrackerUrl={ballotTrackerUrl}
                electionId={electionId}
                ballotId={ballotId}
            />
        </PageLimit>
    )
}

export default ConfirmationScreen
