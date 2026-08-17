// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, CircularProgress, Typography} from "@mui/material"
import React, {useState, useEffect, useContext, useCallback, useRef, useMemo} from "react"
import {useTranslation} from "react-i18next"
import {
    Icon,
    theme,
    ConfirmationLayout,
    Dialog,
    ActionsContainer,
    StyledButton,
} from "@sequentech/ui-essentials"
import {
    stringToHtml,
    IElectionEventPresentation,
    EVotingStatus,
    IAuditableMultiBallot,
    IAuditableSingleBallot,
    EElectionEventContestEncryptionPolicy,
    IElection,
} from "@sequentech/ui-core"
import {styled} from "@mui/material/styles"
import {faPrint} from "@fortawesome/free-solid-svg-icons"
import {useLocation, useNavigate, useParams} from "react-router-dom"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {selectAuditableBallot} from "../store/auditableBallots/auditableBallotsSlice"
import {canVoteSomeElection, CastVoteStatus} from "../store/castVotes/castVotesSlice"
import {selectElectionEventById} from "../store/electionEvents/electionEventsSlice"
import {IElectionExtended} from "../store/elections/electionsSlice"
import {TenantEventType} from ".."
import {clearBallot} from "../store/ballotSelections/ballotSelectionsSlice"
import {
    selectBallotStyleByElectionId,
    selectBallotStyleElectionIds,
    selectFirstBallotStyle,
} from "../store/ballotStyles/ballotStylesSlice"
import {AuthContext} from "../providers/AuthContextProvider"
import {useMutation, useQuery} from "@apollo/client/react"
import {CREATE_BALLOT_RECEIPT} from "../queries/CreateBallotReceipt"
import {useGetPublicDocumentUrl} from "../hooks/public-document-url"
import Stepper from "../components/Stepper"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {provideBallotService} from "../services/BallotService"
import {VotingPortalError, VotingPortalErrorType} from "../services/VotingPortalError"
import {GetCastVotesQuery, GetDocumentQuery, GetElectionsQuery} from "../gql/graphql"
import {GET_ELECTIONS} from "../queries/GetElections"
import {downloadUrl} from "@sequentech/ui-core"
import {
    ConfirmationScreenData,
    selectConfirmationScreenData,
} from "../store/castVotes/confirmationScreenDataSlice"
import {GET_CAST_VOTES} from "../queries/GetCastVotes"
import {GET_DOCUMENT} from "../queries/GetDocument"

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
    isGoldenAuth: boolean
}

const ActionButtons: React.FC<ActionButtonsProps> = ({
    ballotTrackerUrl,
    electionId,
    ballotId,
    isGoldenAuth,
}) => {
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
            electionIds: ballotStyleElectionIds?.length ? ballotStyleElectionIds : [electionId],
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

    const {data: castVotes} = useQuery<GetCastVotesQuery>(GET_CAST_VOTES, {
        skip: globalSettings.DISABLE_AUTH || !isGoldenAuth,
    })

    function isAllowedToCastVote() {
        if (isGoldenAuth) {
            // Can´t use canVote when isGoldenAuth because the state in redux was removed at logout.
            const election = dataElections?.sequent_backend_election.filter(
                (item) => item.id === electionId
            )[0]
            const numAllowedRevotes = election?.num_allowed_revotes ?? 1
            const electionCastVotes =
                castVotes?.sequent_backend_cast_vote.filter(
                    (castVote) =>
                        castVote.election_id === electionId &&
                        castVote.status !== CastVoteStatus.DISCARDED
                ) ?? []
            console.log(numAllowedRevotes, electionCastVotes, election?.id, electionId, castVotes)
            if (numAllowedRevotes === 0) {
                return true
            }

            return electionCastVotes.length < numAllowedRevotes
        } else {
            return canVote
        }
    }

    const onClickFinishButton = useCallback(() => {
        console.log("isGoldenAuth: ", isGoldenAuth)
        console.log(
            "onClickFinishButton",
            isAnyVotingStatusOpen,
            isAllowedToCastVote(),
            canVote,
            globalSettings.DISABLE_AUTH
        )
        if ((isAnyVotingStatusOpen && isAllowedToCastVote()) || globalSettings.DISABLE_AUTH) {
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
                    onClick={onClickFinishButton}
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
    const confirmationScreenData = useAppSelector(selectConfirmationScreenData(String(electionId)))
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
            if (!confirmationScreenData) {
                console.log("confirmationScreenData not found in redux")
                return {ballotIdStored: undefined, isDemoStored: undefined}
            } else {
                return {
                    ballotIdStored: confirmationScreenData.ballotId,
                    isDemoStored: confirmationScreenData.isDemo,
                }
            }
        } else {
            console.log("auditableBallot normal flow")
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
        // The arrangement is `ConfirmationLayout`, in `ui-essentials`, so the
        // Election Architect's preview shows this screen rather than a copy of
        // it. What stays here is what needs the store, the router or this
        // screen's own state: the breadcrumb, the dialogs, and the actions.
        <ConfirmationLayout
            steps={<Stepper selected={3} />}
            title={t("confirmationScreen.title")}
            onTitleHelp={() => setOpenConfirmationHelp(true)}
            description={stringToHtml(t("confirmationScreen.description"))}
            ballotIdLabel={t("confirmationScreen.ballotId")}
            ballotId={ballotId.current ?? ""}
            ballotIdOnPhone={t("ballotHash", {ballotId: ballotId.current})}
            ballotIdHref={isDemo ? undefined : ballotTrackerUrl}
            onBallotIdClick={handleBallotIdLinkClick}
            onBallotIdHelp={() => (isDemo ? setDemoBallotIdHelp(true) : setOpenBallotIdHelp(true))}
            verifyTitle={t("confirmationScreen.verifyCastTitle")}
            verifyDescription={stringToHtml(t("confirmationScreen.verifyCastDescription"))}
            qrValue={isDemo ? t("confirmationScreen.demoQRText") : (ballotTrackerUrl ?? "")}
            actions={
                <ActionButtons
                    ballotTrackerUrl={ballotTrackerUrl}
                    electionId={electionId}
                    ballotId={ballotId.current ?? ""}
                    isGoldenAuth={confirmationScreenData ? true : false}
                />
            }
        >
            {/* The four dialogs, which belong with the state that opens them.
                They sat inside the heading and the identifier's border before;
                MUI renders a dialog into a portal wherever it is declared, so a
                reader cannot tell, and gathering them makes the frame liftable. */}
            <Dialog
                handleClose={() => setOpenConfirmationHelp(false)}
                open={openConfirmationHelp}
                title={t("confirmationScreen.confirmationHelpDialog.title")}
                ok={t("confirmationScreen.confirmationHelpDialog.ok")}
                variant="info"
            >
                {stringToHtml(t("confirmationScreen.confirmationHelpDialog.content"))}
            </Dialog>
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
        </ConfirmationLayout>
    )
}

export default ConfirmationScreen
