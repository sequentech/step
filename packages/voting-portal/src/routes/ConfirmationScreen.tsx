// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, CircularProgress, Typography} from "@mui/material"
import React, {useState, useEffect, useContext, useCallback, useRef} from "react"
import {useTranslation} from "react-i18next"
import {PageLimit, Icon, IconButton, theme, QRCode, Dialog} from "@sequentech/ui-essentials"
import {
    stringToHtml,
    IElectionEventPresentation,
    EVotingStatus,
    IAuditableMultiBallot,
    IAuditableSingleBallot,
    EElectionEventContestEncryptionPolicy,
} from "@sequentech/ui-core"
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
import {clearBallot} from "../store/ballotSelections/ballotSelectionsSlice"
import {
    selectBallotStyleByElectionId,
    selectBallotStyleElectionIds,
    selectFirstBallotStyle,
} from "../store/ballotStyles/ballotStylesSlice"
import {AuthContext} from "../providers/AuthContextProvider"
import {useMutation, useQuery} from "@apollo/client"
import {CREATE_BALLOT_RECEIPT} from "../queries/CreateBallotReceipt"
import {useGetPublicDocumentUrl} from "../hooks/public-document-url"
import Stepper from "../components/Stepper"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {provideBallotService} from "../services/BallotService"
import {VotingPortalError, VotingPortalErrorType} from "../services/VotingPortalError"
import {GetDocumentQuery, GetElectionsQuery} from "../gql/graphql"
import {GET_ELECTIONS} from "../queries/GetElections"
import {downloadUrl} from "@sequentech/ui-core"
import {SessionBallotData} from "../store/castVotes/castVotesSlice"
import {GET_DOCUMENT} from "../queries/GetDocument"

const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
`

const StyledButton = styled(Button)`
    display: flex;
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

const StyledCircularProgress = styled(CircularProgress)`
    width: 14px !important;
    height: 14px !important;
`

const StyledIcon = styled(Icon)`
    min-width: 14px;
    padding: 5px;
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
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const [createBallotReceipt] = useMutation(CREATE_BALLOT_RECEIPT)
    const [documentId, setDocumentId] = useState<string | null>(null)
    const {getDocumentUrl} = useGetPublicDocumentUrl()
    const {globalSettings} = useContext(SettingsContext)
    const [errorDialog, setErrorDialog] = useState<boolean>(false)
    const [openPrintDemoModal, setOpenPrintDemoModal] = useState<boolean>(false)
    const oneBallotStyle = useAppSelector(selectFirstBallotStyle)
    const isDemo = oneBallotStyle?.ballot_eml.public_key?.is_demo
    const [isPolling, setIsPolling] = useState<boolean>(false)

    let presentation = electionEvent?.presentation as IElectionEventPresentation | undefined
    const ballotStyleElectionIds = useAppSelector(selectBallotStyleElectionIds)
    const {data: dataElections} = useQuery<GetElectionsQuery>(GET_ELECTIONS, {
        variables: {
            electionIds: ballotStyleElectionIds,
        },
        skip: globalSettings.DISABLE_AUTH, // Skip query if in demo mode
    })

    const {
        data: ballotReceiptDocuments,
        startPolling,
        stopPolling,
    } = useQuery<GetDocumentQuery>(GET_DOCUMENT, {
        variables: {
            ids: documentId ? [documentId] : [],
            electionEventId: eventId,
            tenantId: tenantId || "",
        },
        skip: !documentId, // Skip query if no documentId
    })

    const isAnyVotingStatusOpen = dataElections?.sequent_backend_election.some(
        (item) => item.status.voting_status === EVotingStatus.OPEN
    )

    const onClickToScreen = useCallback(() => {
        if ((isAnyVotingStatusOpen && canVote) || globalSettings.DISABLE_AUTH) {
            navigate(`/tenant/${tenantId}/event/${eventId}/election-chooser${location.search}`)
        } else {
            logout(presentation?.redirect_finish_url ?? undefined)
        }
    }, [isAnyVotingStatusOpen, canVote])

    useEffect(() => {
        if (ballotStyle) {
            dispatch(clearBallot())
        }
    }, [ballotStyle, dispatch])

    const [isDownloadingReport, setIsDownloadingReport] = useState<boolean>(false)
    const [isHitPrint, setIsHitPrint] = useState<boolean>(false)
    const maxRetries = 5
    const retryInterval = globalSettings.QUERY_POLL_INTERVAL_MS

    async function printBallotReceiptReport() {
        setIsHitPrint(true)
        if (isDemo) {
            setOpenPrintDemoModal(true)
            return
        }
        if (!documentId) {
            console.log("createBallotReceipt")
            const res = await createBallotReceipt({
                variables: {
                    ballot_id: ballotId,
                    ballot_tracker_url: ballotTrackerUrl,
                    election_event_id: eventId,
                    tenant_id: tenantId,
                    election_id: electionId,
                },
            })
            let docId = res.data?.create_ballot_receipt?.id
            console.log("docId: ", docId)
            setDocumentId(docId)
        }
        setIsDownloadingReport(true)
    }

    async function downloadFileWithRetry(url: string, name: string, retries = 0) {
        try {
            await downloadUrl(url, name)
        } catch (error) {
            console.error("Error downloading file:", error)
            if (retries < maxRetries) {
                setTimeout(() => {
                    downloadFileWithRetry(url, name, retries + 1)
                }, retryInterval)
            } else {
                console.error("Failed to download file after", maxRetries, "retries")
            }
        }
    }

    useEffect(() => {
        if (ballotReceiptDocuments?.sequent_backend_document?.[0]?.id && documentId) {
            const fileName = `ballot_receipt_${eventId}.pdf`
            const documentUrl = getDocumentUrl(documentId!, fileName)
            downloadFileWithRetry(documentUrl, fileName)
            setIsDownloadingReport(false)
            setIsHitPrint(false)
            setIsPolling(false)
            setDocumentId(null)
            stopPolling()
        }
    }, [ballotReceiptDocuments?.sequent_backend_document?.[0]?.id, documentId])

    useEffect(() => {
        if (!isPolling && documentId) {
            setIsPolling(true)
            startPolling(globalSettings.QUERY_POLL_INTERVAL_MS)
        }
    }, [startPolling, globalSettings.QUERY_POLL_INTERVAL_MS, documentId, isPolling])

    return (
        <>
            <ActionsContainer>
                <StyledButton
                    onClick={printBallotReceiptReport}
                    disabled={isHitPrint}
                    variant="secondary"
                    sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
                >
                    {isHitPrint ? (
                        <StyledCircularProgress color="inherit" />
                    ) : (
                        <StyledIcon icon={faPrint} size="sm" />
                    )}
                    <Box>{t("confirmationScreen.printButton")}</Box>
                </StyledButton>
                <StyledButton
                    className="finish-button"
                    onClick={onClickToScreen}
                    sx={{width: {xs: "100%", sm: "200px"}}}
                >
                    <Box>{t("confirmationScreen.finishButton")}</Box>
                </StyledButton>
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
                title={t("confirmationScreen.errorDialogPrintBallotReceipt.title")}
                ok={t("confirmationScreen.errorDialogPrintBallotReceipt.ok")}
                variant="warning"
            >
                {stringToHtml(t("confirmationScreen.errorDialogPrintBallotReceipt.content"))}
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
    const {hashBallot, hashMultiBallot} = provideBallotService()
    const oneBallotStyle = useAppSelector(selectFirstBallotStyle)
    const getBallotId = (): {
        ballotIdStored: string | undefined
        isDemoStored: boolean | undefined
    } => {
        if (!auditableBallot) {
            const ballotData = JSON.parse(
                sessionStorage.getItem("ballotData") ?? "{}"
            ) as SessionBallotData
            if (Object.keys(ballotData).length === 0) {
                console.log("ballotData not found in sessionStorage")
                return {ballotIdStored: undefined, isDemoStored: undefined}
            } else {
                return {ballotIdStored: ballotData.ballotId, isDemoStored: ballotData.isDemo}
            }
        } else {
            if (!auditableBallot) {
                console.log("auditableBallot is not there")
                return {ballotIdStored: undefined, isDemoStored: undefined}
            }
            console.log("auditableBallot is there")
            const isMultiContest =
                auditableBallot?.config.election_event_presentation?.contest_encryption_policy ==
                EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS
            const hashableBallot = isMultiContest
                ? hashMultiBallot(auditableBallot as IAuditableMultiBallot)
                : hashBallot(auditableBallot as IAuditableSingleBallot)
            const ballotIdStored = (auditableBallot && hashableBallot) || undefined
            const isDemoStored = oneBallotStyle?.ballot_eml.public_key?.is_demo
            return {ballotIdStored, isDemoStored}
        }
    }

    const ballotId = useRef<string | undefined>(undefined)
    const gotData = useRef<boolean | undefined>(false)
    const navigate = useNavigate()
    const [demoBallotIdHelp, setDemoBallotIdHelp] = useState<boolean>(false)
    const [isDemo, setIsDemo] = useState<boolean>(false)
    const [ballotTrackerUrl, setBallotTrackerUrl] = useState<string | undefined>(undefined)

    if (
        gotData.current &&
        auditableBallot?.ballot_hash &&
        ballotId.current !== auditableBallot?.ballot_hash
    ) {
        console.log(
            `ballotId: ${ballotId.current}\n auditable Ballot Hash: ${auditableBallot?.ballot_hash}`
        )
        throw new VotingPortalError(VotingPortalErrorType.INCONSISTENT_HASH)
    }

    useEffect(() => {
        if (!gotData.current) {
            gotData.current = true
            const {ballotIdStored, isDemoStored} = getBallotId()
            sessionStorage.removeItem("ballotData")
            if (!ballotIdStored) {
                console.log("No stored ballot found, navigating to the election-chooser page.")
                navigate(`/tenant/${tenantId}/event/${eventId}/election-chooser`)
            }
            ballotId.current = ballotIdStored
            setIsDemo(isDemoStored ?? false)
            setBallotTrackerUrl(
                `${window.location.protocol}//${window.location.host}/tenant/${tenantId}/event/${eventId}/election/${electionId}/ballot-locator/${ballotIdStored}`
            )
        }
    }, [])

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
                        {ballotId.current}
                    </BallotIdLink>
                    <BallotIdLink
                        href={!isDemo ? ballotTrackerUrl : undefined}
                        target={!isDemo ? "_blank" : undefined}
                        sx={{display: {xs: "block", sm: "none"}}}
                        onClick={handleBallotIdLinkClick}
                    >
                        {t("ballotHash", {ballotId: ballotId.current})}
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
                <QRCode
                    value={isDemo ? t("confirmationScreen.demoQRText") : ballotTrackerUrl ?? ""}
                />
            </QRContainer>
            <ActionButtons
                ballotTrackerUrl={ballotTrackerUrl}
                electionId={electionId}
                ballotId={ballotId.current ?? ""}
            />
        </PageLimit>
    )
}

export default ConfirmationScreen
