// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Alert} from "@mui/material"
import React, {useState, useEffect, useContext, useCallback, useRef, useMemo} from "react"
import {useTranslation} from "react-i18next"
import {
    ConfirmationLayout,
    Dialog,
    ConfirmationActions,
    StyledButton,
    VisuallyHidden,
    theme,
} from "@sequentech/ui-essentials"
import {
    stringToHtml,
    IElectionEventPresentation,
    IAuditableMultiBallot,
    IAuditableSingleBallot,
    EElectionEventContestEncryptionPolicy,
    IElection,
    areAllContestsAcclaimed,
    EVotingPortalAuditButtonCfg,
} from "@sequentech/ui-core"
import {useLocation, useNavigate, useParams} from "react-router-dom"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {
    selectAuditableBallot,
    selectIsBlankBallot,
} from "../store/auditableBallots/auditableBallotsSlice"
import {canVoteElection} from "../store/castVotes/castVotesSlice"
import {useVoterContext} from "../hooks/useVoterContext"
import {isElectionOpenForVoting} from "../services/VotingAvailability"
import {selectElectionEventById} from "../store/electionEvents/electionEventsSlice"
import {TenantEventType} from ".."
import {clearBallot} from "../store/ballotSelections/ballotSelectionsSlice"
import {
    selectBallotStyleByElectionId,
    selectFirstBallotStyle,
} from "../store/ballotStyles/ballotStylesSlice"
import {AuthContext} from "../providers/AuthContextProvider"
import {useMutation} from "@apollo/client/react"
import {CREATE_BALLOT_RECEIPT} from "../queries/CreateBallotReceipt"
import {useReceiptDocument} from "../hooks/useReceiptDocument"
import Stepper from "../components/Stepper"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {provideBallotService} from "../services/BallotService"
import {VotingPortalError, VotingPortalErrorType} from "../services/VotingPortalError"
import {
    CreateBallotReceiptMutation,
    CreateBallotReceiptMutationVariables,
    GetDocumentQuery,
} from "../gql/graphql"
import {downloadUrl, formatVotingPortalDateTime} from "@sequentech/ui-core"
import {
    ConfirmationScreenData,
    selectConfirmationScreenData,
} from "../store/castVotes/confirmationScreenDataSlice"
import {GET_DOCUMENT} from "../queries/GetDocument"
import {BallotStatus, EBallotStatusTrigger} from "../components/Vitrine/BallotStatus"
import {BallotTag} from "../components/Vitrine/BallotTag"
import {GetUpdates} from "../components/Vitrine/GetUpdates"
import {ID_BUTTON_SX} from "../components/Vitrine/idButtons"
import {useCastReceipt} from "../hooks/useCastReceipt"
import {useElectionName} from "../hooks/useElectionName"
import {groupBallotId, shortBallotId} from "../services/vitrine/ballotId"
import {EBallotTag} from "../services/vitrine/ballotStatus"
import type {IReceiptView} from "../services/vitrine/castReceipt"
import {EHubUpdate, offeredHubs, readHubUpdates} from "../services/vitrine/hubUpdates"
import {ETransparencyMode} from "../services/vitrine/types"
import {selectReceivedBallot} from "../store/receivedBallots/receivedBallotsSlice"

/*
 * The spinner and the printer were here, with the row they belonged to.
 *
 * Both are `ConfirmationActions` in `ui-essentials` now, so the Election Architect's
 * preview draws this screen's buttons rather than two plain ones of its own.
 */

interface ActionButtonsProps {
    electionId?: string
    ballotTrackerUrl?: string
    ballotId: string
    isFullyAcclaimed: boolean
}

const ActionButtons: React.FC<ActionButtonsProps> = ({
    ballotTrackerUrl,
    electionId,
    ballotId,
    isFullyAcclaimed,
}) => {
    const {isKiosk, logout} = useContext(AuthContext)
    const {t} = useTranslation()
    const {tenantId, eventId} = useParams<TenantEventType>()
    const storedCastVotes = useAppSelector((state) => state.castVotes)
    const completedAcclaimedElections = useAppSelector(
        (state) => state.extra.completedAcclaimedElections
    )
    const navigate = useNavigate()
    const location = useLocation()
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionId)))
    const dispatch = useAppDispatch()
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const [createBallotReceipt] = useMutation<
        CreateBallotReceiptMutation,
        CreateBallotReceiptMutationVariables
    >(CREATE_BALLOT_RECEIPT)
    const [documentId, setDocumentId] = useState<string | null>(null)
    const {globalSettings} = useContext(SettingsContext)
    const [errorDialog, setErrorDialog] = useState<boolean>(false)
    const [openPrintDemoModal, setOpenPrintDemoModal] = useState<boolean>(false)
    const oneBallotStyle = useAppSelector(selectFirstBallotStyle)
    const isDemo = oneBallotStyle?.ballot_eml.public_key?.is_demo
    const [isPolling, setIsPolling] = useState<boolean>(false)

    let presentation = electionEvent?.presentation as IElectionEventPresentation | undefined
    // Eligibility includes elections whose ballots have never been downloaded,
    // including after a direct entry or golden-policy reauthentication.
    const voterContext = useVoterContext()

    const {
        data: ballotReceiptDocuments,
        error: receiptError,
        startPolling,
        stopPolling,
    } = useReceiptDocument(eventId, documentId)

    const canVote = voterContext.data?.sequent_backend_election.some((election) => {
        // Bootstrap includes earlier votes, while Redux also has the cast that
        // just completed. Merge by ID so cached metadata neither loses nor
        // double-counts that vote after normal or golden authentication.
        const votes = new Map<string, {status?: string | null}>()
        for (const vote of voterContext.data?.cast_votes ?? []) {
            if (vote.election_id === election.id) votes.set(vote.id, vote)
        }
        for (const vote of storedCastVotes[election.id] || []) votes.set(vote.id, vote)
        return (
            canVoteElection(
                election,
                Array.from(votes.values()),
                completedAcclaimedElections?.[election.id]
            ) &&
            isElectionOpenForVoting({
                electionStatus: election.status,
                eventStatus: voterContext.data?.sequent_backend_election_event[0]?.status,
                channels: election.voting_channels,
                areaPresentation: voterContext.summaries?.[election.id]?.area_presentation,
                isKiosk: isKiosk(),
            })
        )
    })
    const waitingForEligibility =
        !globalSettings.DISABLE_AUTH && (voterContext.loading || !voterContext.data)

    const onClickFinishButton = useCallback(() => {
        if (waitingForEligibility) return
        if (canVote || globalSettings.DISABLE_AUTH) {
            navigate(`/tenant/${tenantId}/event/${eventId}/election-chooser${location.search}`)
        } else {
            const redirectUrl = isKiosk()
                ? presentation?.kiosk_redirect_finish_url
                : presentation?.redirect_finish_url
            logout(redirectUrl)
        }
    }, [
        waitingForEligibility,
        canVote,
        globalSettings.DISABLE_AUTH,
        navigate,
        tenantId,
        eventId,
        location.search,
        isKiosk,
        presentation,
        logout,
    ])

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
        try {
            setErrorDialog(false)
            if (!documentId) {
                if (!ballotTrackerUrl) {
                    setIsHitPrint(false)
                    return
                }
                const res = await createBallotReceipt({
                    variables: {
                        ballot_id: ballotId,
                        ballot_tracker_url: ballotTrackerUrl,
                        election_event_id: eventId,
                        tenant_id: tenantId,
                        election_id: electionId,
                    },
                })
                const docId = res.data?.create_ballot_receipt?.id
                if (!docId) throw new Error("Receipt generation returned no document")
                setDocumentId(docId)
            }
            setIsDownloadingReport(true)
        } catch {
            failReceipt()
        }
    }

    const failReceipt = useCallback(() => {
        stopPolling()
        setDocumentId(null)
        setIsPolling(false)
        setIsDownloadingReport(false)
        setIsHitPrint(false)
        setErrorDialog(true)
    }, [stopPolling])

    useEffect(() => {
        if (documentId && receiptError) failReceipt()
    }, [documentId, receiptError, failReceipt])

    useEffect(() => {
        if (!documentId) return
        const timeout = setTimeout(failReceipt, globalSettings.POLLING_DURATION_TIMEOUT)
        return () => clearTimeout(timeout)
    }, [documentId, globalSettings.POLLING_DURATION_TIMEOUT, failReceipt])

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
        if (
            documentId &&
            !receiptError &&
            ballotReceiptDocuments?.receipt_document.payload.document?.id === documentId
        ) {
            const fileName = `ballot_receipt_${eventId}.pdf`
            const documentUrl = ballotReceiptDocuments.receipt_document.payload.document.url
            downloadFileWithRetry(documentUrl, fileName)
            setIsDownloadingReport(false)
            setIsHitPrint(false)
            setIsPolling(false)
            setDocumentId(null)
            stopPolling()
        }
    }, [ballotReceiptDocuments?.receipt_document.payload.document?.id, documentId, receiptError])

    useEffect(() => {
        if (!isPolling && documentId && !receiptError) {
            setIsPolling(true)
            startPolling(globalSettings.QUERY_POLL_INTERVAL_MS)
        }
    }, [startPolling, globalSettings.QUERY_POLL_INTERVAL_MS, documentId, isPolling, receiptError])

    return (
        <>
            {voterContext.error && !globalSettings.DISABLE_AUTH ? (
                <Alert
                    severity="error"
                    className="remaining-elections-error"
                    action={
                        <StyledButton
                            className="remaining-elections-retry-button"
                            variant="secondary"
                            onClick={() => void voterContext.retry()}
                            disabled={voterContext.loading}
                        >
                            {t("confirmationScreen.retryButton")}
                        </StyledButton>
                    }
                >
                    {t("confirmationScreen.remainingElectionsError")}
                </Alert>
            ) : null}
            <VisuallyHidden className="remaining-elections-status" role="status">
                {waitingForEligibility && !voterContext.error ? t("a11y.loading") : ""}
            </VisuallyHidden>
            {/* `ConfirmationActions` in `ui-essentials`: the secondary Print with its
                printer, and Finish. Here so the Election Architect's preview draws this
                row rather than two plain buttons — what Print *does* stays with this
                route, since it renders a receipt from a cast vote. There is no ballot
                to receipt when nothing was cast, and no receipt to make without a
                document or a tracker URL. */}
            <ConfirmationActions
                withPrint={!isFullyAcclaimed}
                printing={isHitPrint}
                onPrint={
                    isDemo || documentId || ballotTrackerUrl ? printBallotReceiptReport : undefined
                }
                onFinish={
                    waitingForEligibility || voterContext.error ? undefined : onClickFinishButton
                }
            />

            <Dialog
                className="demo-print-receipt-dialog"
                handleClose={() => setOpenPrintDemoModal(false)}
                open={openPrintDemoModal}
                title={t("confirmationScreen.demoPrintDialog.title")}
                ok={t("confirmationScreen.demoPrintDialog.ok")}
                variant="info"
            >
                {stringToHtml(t("confirmationScreen.demoPrintDialog.content"))}
            </Dialog>
            <Dialog
                className="print-receipt-error-dialog"
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

interface ConfirmationBallotStatusProps {
    eventId: string
    electionId: string
    ballotId: string
    /** The receipt of the ballot just cast, as far as it has been checked. */
    view?: IReceiptView
    /** Whether this device keeps the receipt. */
    kept: boolean
    trackerUrl?: string
}

/** The check mark in the Tracking ID box: the status of the ballot just cast,
 * which its receipt upgrades while this page is open. */
const ConfirmationBallotStatus: React.FC<ConfirmationBallotStatusProps> = ({
    eventId,
    electionId,
    ballotId,
    view,
    kept,
    trackerUrl,
}) => {
    const {i18n} = useTranslation()
    const electionName = useElectionName(electionId)
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const acceptedAt = view?.own?.acceptedAt
    const castAt =
        acceptedAt === undefined
            ? undefined
            : formatVotingPortalDateTime(
                  acceptedAt * 1000,
                  electionEvent,
                  i18n.resolvedLanguage || i18n.language
              )

    return (
        <BallotStatus
            trigger={EBallotStatusTrigger.ICON}
            view={view}
            ballotId={ballotId}
            electionName={electionName}
            castAt={castAt}
            kept={kept}
            trackerUrl={trackerUrl}
            surface={theme.palette.green.light}
            sx={{m: "-7px -4px -7px -7px"}}
        />
    )
}

const ConfirmationScreen: React.FC = () => {
    const {isKiosk} = useContext(AuthContext)
    const {tenantId, eventId} = useParams<TenantEventType>()
    const {electionId} = useParams<{electionId?: string}>()
    const auditableBallot = useAppSelector(selectAuditableBallot(String(electionId)))
    const isBlankBallot = useAppSelector(selectIsBlankBallot(String(electionId)))
    const confirmationScreenData = useAppSelector(selectConfirmationScreenData(String(electionId)))
    const receivedBallot = useAppSelector(selectReceivedBallot(String(electionId)))
    const {t} = useTranslation()
    const [openBallotIdHelp, setOpenBallotIdHelp] = useState(false)
    const [openConfirmationHelp, setOpenConfirmationHelp] = useState(false)
    const [openDemoBallotUrlHelp, setDemoBallotUrlHelp] = useState(false)
    const {hashBallot, hashMultiBallot} = provideBallotService()
    const oneBallotStyle = useAppSelector(selectFirstBallotStyle)
    const electionBallotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionId)))
    const auditButtonCfg =
        electionBallotStyle?.ballot_eml?.election_presentation?.audit_button_cfg ??
        confirmationScreenData?.auditButtonCfg ??
        EVotingPortalAuditButtonCfg.SHOW
    // Nothing was cast for a fully acclaimed election, so this screen confirms
    // what was decided rather than a ballot, and shows no ballot id anywhere.
    const isFullyAcclaimed = areAllContestsAcclaimed(electionBallotStyle?.ballot_eml.contests)

    const getBallotId = (): {
        ballotIdStored: string | undefined
        ballotHashStored: string | undefined
        isDemoStored: boolean | undefined
    } => {
        if (!auditableBallot) {
            if (!confirmationScreenData) {
                console.log("confirmationScreenData not found in redux")
                return {
                    ballotIdStored: undefined,
                    ballotHashStored: undefined,
                    isDemoStored: undefined,
                }
            } else {
                return {
                    ballotIdStored: confirmationScreenData.ballotId,
                    ballotHashStored: undefined,
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
            const ballotHashStored = (auditableBallot && hashableBallot) || undefined
            // A ballot the ballot box received at review is known by the
            // Ballot ID the ballot box signed for it.
            const ballotIdStored =
                receivedBallot && receivedBallot.ballot_hash === ballotHashStored
                    ? receivedBallot.ballot_id
                    : ballotHashStored
            const isDemoStored = oneBallotStyle?.ballot_eml.public_key?.is_demo
            return {ballotIdStored, ballotHashStored, isDemoStored}
        }
    }

    const ballotId = useRef<string | undefined>(undefined)
    const ballotHash = useRef<string | undefined>(undefined)
    const gotData = useRef<boolean | undefined>(false)
    const navigate = useNavigate()
    const [demoBallotIdHelp, setDemoBallotIdHelp] = useState<boolean>(false)
    const [isDemo, setIsDemo] = useState<boolean>(false)
    const [ballotTrackerUrl, setBallotTrackerUrl] = useState<string | undefined>(undefined)

    if (
        gotData.current &&
        auditableBallot?.ballot_hash &&
        ballotHash.current !== auditableBallot?.ballot_hash
    ) {
        console.log(
            `ballotId: ${ballotHash.current}\n auditable Ballot Hash: ${auditableBallot?.ballot_hash}`
        )
        throw new VotingPortalError(VotingPortalErrorType.INCONSISTENT_HASH)
    }

    useEffect(() => {
        if (isFullyAcclaimed) {
            return
        }
        if (!gotData.current) {
            gotData.current = true
            const {ballotIdStored, ballotHashStored, isDemoStored} = getBallotId()
            if (!ballotIdStored) {
                console.log("No stored ballot found, navigating to the election-chooser page.")
                navigate(`/tenant/${tenantId}/event/${eventId}/election-chooser`)
            }
            ballotId.current = ballotIdStored
            ballotHash.current = ballotHashStored
            setIsDemo(isDemoStored ?? false)
            setBallotTrackerUrl(
                `${window.location.protocol}//${window.location.host}/tenant/${tenantId}/event/${eventId}/election/${electionId}/ballot-locator/${ballotIdStored}${isKiosk() ? "?kiosk" : ""}`
            )
        }
    }, [])

    const transparency = confirmationScreenData?.vitrine
    const vitrine =
        transparency !== undefined &&
        transparency.transparency.mode !== ETransparencyMode.DISABLED &&
        !isDemo &&
        !isFullyAcclaimed &&
        eventId &&
        electionId &&
        ballotId.current
            ? transparency
            : undefined

    // The receipt of the ballot just cast, shared by the check mark in the
    // Tracking ID box and by the hubs' links, which carry it to their sites.
    const {view, kept} = useCastReceipt(
        {eventId: eventId ?? "", electionId: electionId ?? "", trackingId: ballotId.current ?? ""},
        vitrine
    )
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const hubs = useMemo(() => offeredHubs(view?.policy), [view?.policy])
    // Not on a kiosk: the hub's address, with the receipt in it, would stay in
    // the shared browser's history, and a kiosk keeps no receipts for that reason.
    const offersUpdates =
        vitrine !== undefined &&
        !isKiosk() &&
        readHubUpdates(electionEvent?.presentation).confirmation === EHubUpdate.ON &&
        hubs.length > 0

    const handleBallotIdLinkClick = (event: React.MouseEvent<HTMLAnchorElement, MouseEvent>) => {
        if (isDemo) {
            event.preventDefault()
            setDemoBallotUrlHelp(true)
        } else if (isKiosk() && ballotTrackerUrl) {
            // Keep the in-memory kiosk session; a new tab starts a fresh login.
            event.preventDefault()
            const {pathname, search} = new URL(ballotTrackerUrl)
            navigate({pathname, search})
        }
    }

    return (
        // The arrangement is `ConfirmationLayout`, in `ui-essentials`, so the
        // Election Architect's preview shows this screen rather than a copy of
        // it. What stays here is what needs the store, the router or this
        // screen's own state: the breadcrumb, the dialogs, and the actions.
        <ConfirmationLayout
            steps={<Stepper selected={3} />}
            onTitleHelp={() => setOpenConfirmationHelp(true)}
            isFullyAcclaimed={isFullyAcclaimed}
            isBlankBallot={isBlankBallot}
            ballotId={ballotId.current ?? ""}
            ballotIdOnPhone={t("ballotHash", {ballotId: ballotId.current})}
            {...(vitrine && eventId && electionId
                ? {
                      shortBallotId: shortBallotId(ballotId.current ?? ""),
                      ballotIdTag: <BallotTag tag={EBallotTag.CAST} />,
                      ballotIdCopyValue: groupBallotId(ballotId.current ?? ""),
                      ballotIdStatus: (
                          <ConfirmationBallotStatus
                              eventId={eventId}
                              electionId={electionId}
                              ballotId={ballotId.current ?? ""}
                              view={view}
                              kept={kept}
                              trackerUrl={ballotTrackerUrl}
                          />
                      ),
                      ballotIdButtonSx: ID_BUTTON_SX,
                      ballotIdCopiedColor: theme.palette.green.dark,
                      compact: true,
                  }
                : {})}
            ballotIdHref={isDemo ? undefined : ballotTrackerUrl}
            ballotIdTarget={isKiosk() ? null : "_blank"}
            onBallotIdClick={handleBallotIdLinkClick}
            onBallotIdHelp={() => (isDemo ? setDemoBallotIdHelp(true) : setOpenBallotIdHelp(true))}
            ballotIdCopyLabels={
                auditButtonCfg !== EVotingPortalAuditButtonCfg.NOT_SHOW
                    ? {
                          copy: t(vitrine ? "vitrine.ballotId.copy" : "reviewScreen.copyBallotId"),
                          copied: t(
                              vitrine ? "vitrine.ballotId.copied" : "reviewScreen.ballotIdCopied"
                          ),
                          error: t(
                              vitrine
                                  ? "vitrine.ballotId.copyByHand"
                                  : "reviewScreen.ballotIdCopyError"
                          ),
                      }
                    : undefined
            }
            qrValue={isDemo ? t("confirmationScreen.demoQRText") : (ballotTrackerUrl ?? "")}
            actions={
                <ActionButtons
                    ballotTrackerUrl={ballotTrackerUrl}
                    electionId={electionId}
                    ballotId={ballotId.current ?? ""}
                    isFullyAcclaimed={isFullyAcclaimed}
                />
            }
        >
            {/* "Get email updates", between the QR code and the actions: the layout
                renders its children there, after the QR code and before the
                actions. The order of the tiles is drawn when the callout
                appears, so a new policy asks for a new draw with its key. */}
            {offersUpdates && view?.policy ? (
                <GetUpdates
                    key={view.policy.id}
                    hubs={hubs}
                    policyId={view.policy.id}
                    receipt={view.chain}
                />
            ) : null}
            {/* The four dialogs, which belong with the state that opens them.
                They sat inside the heading and the identifier's border before;
                MUI renders a dialog into a portal wherever it is declared, so a
                reader cannot tell, and gathering them makes the frame liftable. */}
            <Dialog
                className="screen-help-dialog confirmation-help-dialog"
                handleClose={() => setOpenConfirmationHelp(false)}
                open={openConfirmationHelp}
                title={t(
                    isFullyAcclaimed
                        ? "confirmationScreen.acclamation.helpDialog.title"
                        : "confirmationScreen.confirmationHelpDialog.title"
                )}
                ok={t(
                    isFullyAcclaimed
                        ? "confirmationScreen.acclamation.helpDialog.ok"
                        : "confirmationScreen.confirmationHelpDialog.ok"
                )}
                variant="info"
            >
                {stringToHtml(
                    t(
                        isFullyAcclaimed
                            ? "confirmationScreen.acclamation.helpDialog.content"
                            : "confirmationScreen.confirmationHelpDialog.content"
                    )
                )}
            </Dialog>
            <Dialog
                className="ballot-id-help-dialog"
                handleClose={() => setOpenBallotIdHelp(false)}
                open={openBallotIdHelp}
                title={t("confirmationScreen.ballotIdHelpDialog.title")}
                ok={t("confirmationScreen.ballotIdHelpDialog.ok")}
                variant="info"
            >
                {stringToHtml(t("confirmationScreen.ballotIdHelpDialog.content"))}
            </Dialog>
            <Dialog
                className="demo-ballot-url-dialog"
                handleClose={() => setDemoBallotUrlHelp(false)}
                open={openDemoBallotUrlHelp}
                title={t("confirmationScreen.demoBallotUrlDialog.title")}
                ok={t("confirmationScreen.demoBallotUrlDialog.ok")}
                variant="info"
            >
                {stringToHtml(t("confirmationScreen.demoBallotUrlDialog.content"))}
            </Dialog>
            <Dialog
                className="demo-ballot-id-help-dialog"
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
