// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState, useContext, useMemo, useRef} from "react"
import {useStore} from "react-redux"
import type {RootState} from "../store/store"
import {castVoteErrorMessage} from "../services/CastVoteErrors"
import {EReceiveBallotStatus, useReceiveBallot} from "../hooks/useReceiveBallot"
import {
    IReceivedCast,
    signReceivedCast,
    useCastReceivedBallot,
} from "../hooks/useCastReceivedBallot"
import {selectReceivedBallot} from "../store/receivedBallots/receivedBallotsSlice"
import {
    Link as RouterLink,
    useNavigate,
    useParams,
    redirect,
    useLocation,
    useSubmit,
} from "react-router-dom"
import {IBallotStyle, selectBallotStyleByElectionId} from "../store/ballotStyles/ballotStylesSlice"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {Box, CircularProgress} from "@mui/material"
import {useBallotStyleSlates} from "../hooks/useBallotStyleSlates"
import {
    BallotSlatesProvider,
    Dialog,
    EWarnBoxAnnouncement,
    Icon,
    ReviewActions,
    ReviewLayout,
    WarnBox,
} from "@sequentech/ui-essentials"
import {
    stringToHtml,
    escapeTranslationValues,
    IAuditableBallot,
    EVotingPortalAuditButtonCfg,
    IAuditableSingleBallot,
    IAuditableMultiBallot,
    ECastVoteGoldLevelPolicy,
    EElectionEventContestEncryptionPolicy,
    IHashableBallot,
    areAllContestsAcclaimed,
} from "@sequentech/ui-core"
import {styled} from "@mui/material/styles"
import {faAngleLeft, faFire} from "@fortawesome/free-solid-svg-icons"
import {useTranslation} from "react-i18next"
import Button from "@mui/material/Button"
import {
    clearAuditableBallot,
    EBallotAuditStatus,
    selectAuditableBallot,
    selectBallotAuditStatus,
} from "../store/auditableBallots/auditableBallotsSlice"
import {useMutation, useQuery} from "@apollo/client/react"
import {INSERT_CAST_VOTE, InsertCastVoteWithVitrineMutation} from "../queries/InsertCastVote"
import {GetElectionEventQuery, GetElectionsQuery} from "../gql/graphql"
import {GET_ELECTIONS} from "../queries/GetElections"
import {provideBallotService} from "../services/BallotService"
import {orderContestsForReview} from "../services/ContestsOrder"
import {ICastVote, addCastVotes} from "../store/castVotes/castVotesSlice"
import {TenantEventType} from ".."
import {useRootBackLink} from "../hooks/root-back-link"
import {
    CastBallotsErrorType,
    VotingPortalErrorType,
    WasmCastBallotsErrorType,
} from "../services/VotingPortalError"
import {IBallotError} from "../types/errors"
import Stepper from "../components/Stepper"
import {selectBallotSelectionByElectionId} from "../store/ballotSelections/ballotSelectionsSlice"
import {
    hashBallot,
    hashMultiBallot,
    IHashableSingleBallot,
    IHashableMultiBallot,
} from "@sequentech/ui-core"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {AuthContext} from "../providers/AuthContextProvider"
import {
    SessionBallotData,
    clearSessionStorageBallotData,
    BALLOT_DATA_KEY,
    BALLOT_DATA_EXPIRATION_KEY,
    SessionBallotSnapshot,
    snapshotSessionBallotData,
    isCurrentSessionBallotData,
    readSessionBallotLocator,
    SessionBallotLocator,
} from "../store/castVotes/sessionBallotData"
import {
    IConfirmationVitrine,
    setConfirmationScreenData,
} from "../store/castVotes/confirmationScreenDataSlice"
import {asTransparency, eventTransparency, ITransparency} from "../services/vitrine/presentation"
import {ETransparencyMode, IVitrineCast} from "../services/vitrine/types"
import {ReviewBallotId} from "../components/Vitrine/ReviewBallotId"
import {useElectionName} from "../hooks/useElectionName"
import {selectElectionById} from "../store/elections/electionsSlice"
import {completeAcclaimedElection, isDeclineToVoteByElectionId} from "../store/extra/extraSlice"
import {isSameBallotSelection} from "../services/BallotSelectionComparison"
import {getBallotReviewSummary} from "../services/ReviewSummary"
import {ReviewContestFooter} from "../components/ReviewSummary/ReviewContestFooter"
import {ReviewSelectionSummary} from "../components/ReviewSummary/ReviewSelectionSummary"
import UnfilledContestsDialog from "../components/UnfilledContestsDialog/UnfilledContestsDialog"
import {getUnfilledContests, IUnfilledContest} from "../services/UnfilledContests"

const StyledButton = styled(Button)`
    display: flex;
    padding: 5px;

    span {
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        padding: 5px;
    }
` as typeof Button

const BallotIdHelpDialog = styled(Dialog)`
    @media (min-width: 601px) {
        .MuiDialogActions-root.has-middle > .cancel-button,
        .MuiDialogActions-root.has-middle > .audit-button {
            flex: 1 1 0;
            min-width: 0;
            width: auto;
        }
    }
`

interface AuditButtonProps {
    onClick: () => void
}

const AuditButton: React.FC<AuditButtonProps> = ({onClick}) => {
    const {t} = useTranslation()

    return (
        <StyledButton
            className="audit-button"
            sx={{width: {xs: "100%", sm: "200px"}}}
            variant="warning"
            onClick={onClick}
        >
            <Icon className="audit-button-icon" icon={faFire} size="sm" />
            <Box className="audit-button-label">{t("reviewScreen.auditButton")}</Box>
        </StyledButton>
    )
}

interface AuditBallotHelpDialogProps {
    auditBallotHelp: boolean
    handleClose: (value: boolean) => void
}

const AuditBallotHelpDialog: React.FC<AuditBallotHelpDialogProps> = ({
    auditBallotHelp,
    handleClose,
}) => {
    const {t} = useTranslation()

    return (
        <Dialog
            className="audit-ballot-dialog"
            handleClose={handleClose}
            open={auditBallotHelp}
            title={t("reviewScreen.auditBallotHelpDialog.title")}
            ok={t("reviewScreen.auditBallotHelpDialog.ok")}
            cancel={t("reviewScreen.auditBallotHelpDialog.cancel")}
            variant="warning"
            maxWidth="md"
        >
            {stringToHtml(t("reviewScreen.auditBallotHelpDialog.content"))}
        </Dialog>
    )
}

/*
 * `LoadingOrCastButton` was here, and the whole row with it.
 *
 * Both are `ReviewActions` in `ui-essentials` now — the spinner, the chevron, the flame on
 * *Audit ballot* and its warning colour — so the Election Architect's preview shows this
 * row instead of three plain buttons of its own. `AuditButton` above stays: the
 * ballot-identifier help dialog puts one inside itself when the audit policy says
 * `SHOW_IN_HELP`, which is not part of the row.
 */

const useAddFakeCastVote = (tenantId: string | undefined, eventId: string | undefined) => {
    const dispatch = useAppDispatch()
    return () => {
        const newCastVote: ICastVote = {
            id: eventId ?? "",
            tenant_id: tenantId ?? "",
            election_id: eventId,
            area_id: eventId,
            created_at: null,
            last_updated_at: null,
            annotations: null,
            labels: null,
            content: "",
            cast_ballot_signature: "",
            voter_id_string: null,
            election_event_id: eventId ?? "",
        }
        console.log("faking casting demo vote")
        dispatch(addCastVotes([newCastVote]))
    }
}

type TCastOutcome = {ok: false} | {ok: true; vitrine: IVitrineCast | null}

/** Vitrine's part of the confirmation, for events with transparency on. */
const confirmationVitrine = (
    transparency: ITransparency | undefined,
    vitrine: IVitrineCast | null
): IConfirmationVitrine | undefined =>
    transparency && transparency.mode !== ETransparencyMode.DISABLED
        ? {cast: vitrine, transparency}
        : undefined

/** What the cast of a received ballot reports, in the shape of the other cast. */
const receivedCastOutcome = (cast: boolean): TCastOutcome =>
    cast ? {ok: true, vitrine: null} : {ok: false}

const useTryInsertCastVote = () => {
    const {t} = useTranslation()
    const [insertCastVote] = useMutation<InsertCastVoteWithVitrineMutation>(INSERT_CAST_VOTE)
    const dispatch = useAppDispatch()
    const store = useStore<RootState>()

    return async (
        electionId: string,
        ballotId: string,
        content: string,
        reportError: (msg: string) => void,
        isCurrent: () => boolean = () => true,
        transparency?: ITransparency
    ): Promise<TCastOutcome> => {
        const setErrorMsg = (message: string) => {
            if (isCurrent()) reportError(message)
        }
        try {
            let result = await insertCastVote({
                variables: {
                    electionId,
                    ballotId,
                    content,
                },
            })

            if (result.error) {
                console.log(result.error?.message)
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.UNABLE_TO_FETCH_DATA}`))
                return {ok: false}
            }

            if (!isCurrent()) return {ok: false}
            let newCastVote = result.data?.insert_cast_vote
            if (newCastVote) {
                const {vitrine, ...castVote} = newCastVote
                dispatch(addCastVotes([castVote]))
                const confirmation = confirmationVitrine(transparency, vitrine ?? null)
                dispatch(
                    setConfirmationScreenData({
                        electionId,
                        confirmationScreenData: {
                            ballotId,
                            isDemo: false,
                            ...(confirmation ? {vitrine: confirmation} : {}),
                        },
                    })
                )
                const currentBallot = store.getState().auditableBallots[electionId]?.auditableBallot
                if (!currentBallot || currentBallot.ballot_hash === ballotId) {
                    dispatch(clearAuditableBallot(electionId))
                    provideBallotService().forgetVoterSigningKey(electionId)
                }
            }
            return newCastVote ? {ok: true, vitrine: newCastVote.vitrine ?? null} : {ok: false}
        } catch (error) {
            console.log(error)
            setErrorMsg(castVoteErrorMessage(error, t))
            return {ok: false}
        }
    }
}

// The ref rejects a second cast before the next render; the state renders the
// pending cast.
const useCastingState = () => {
    const castingRef = useRef<boolean>(false)
    const [isCasting, setIsCasting] = useState<boolean>(false)
    const setCasting = (value: boolean) => {
        castingRef.current = value
        setIsCasting(value)
    }
    return {castingRef, isCasting, setCasting}
}

interface ActionButtonProps {
    ballotStyle: IBallotStyle
    auditableBallot?: IAuditableBallot
    auditButtonCfg: EVotingPortalAuditButtonCfg
    castVoteConfirmModal: boolean
    ballotId: string
    setErrorMsg: (msg: string) => void
    isGoldenPolicy: boolean
    isMultiContest: boolean
    isDeclineToVote: boolean
    isBlankBallot: boolean
    isFullyAcclaimed: boolean
    hasInconsistentHash: boolean
    // The ballot box has not yet stored and signed this ballot.
    isAwaitingReceipt: boolean
    receivedBallotId?: string
    unfilledContests: Array<IUnfilledContest>
}

const ActionButtons: React.FC<ActionButtonProps> = ({
    ballotStyle,
    auditableBallot,
    auditButtonCfg,
    castVoteConfirmModal,
    ballotId,
    setErrorMsg,
    isGoldenPolicy,
    isMultiContest,
    isDeclineToVote,
    isBlankBallot,
    isFullyAcclaimed,
    hasInconsistentHash,
    isAwaitingReceipt,
    receivedBallotId,
    unfilledContests,
}) => {
    const castRefused = hasInconsistentHash || isAwaitingReceipt
    const {t} = useTranslation()
    const navigate = useNavigate()
    const location = useLocation()
    const [auditBallotHelp, setAuditBallotHelp] = useState<boolean>(false)
    const {castingRef, isCasting, setCasting} = useCastingState()
    const [isConfirmCastVoteModal, setConfirmCastVoteModal] = React.useState<boolean>(false)
    const [isUnfilledContestsModal, setUnfilledContestsModal] = useState<boolean>(false)
    const {tenantId, eventId} = useParams<TenantEventType>()
    const {toHashableBallot, toHashableMultiBallot} = provideBallotService()
    const submit = useSubmit()
    const isDemo = !!ballotStyle?.ballot_eml?.public_key?.is_demo
    const {globalSettings} = useContext(SettingsContext)
    const authContext = useContext(AuthContext)
    const {isGoldUser, reauthWithGold} = authContext
    const addFakeCastVote = useAddFakeCastVote(tenantId, eventId)
    const tryInsertCastVote = useTryInsertCastVote()
    const tryCastReceivedBallot = useCastReceivedBallot()
    const storedReceivedBallot = useAppSelector(selectReceivedBallot(ballotStyle.election_id))
    const receivedBallot =
        receivedBallotId && storedReceivedBallot?.ballot_id === receivedBallotId
            ? storedReceivedBallot
            : undefined
    const dispatch = useAppDispatch()
    const events = useAppSelector((state) => state.electionEvent)
    const transparency = eventTransparency(
        events[ballotStyle.election_event_id]?.presentation,
        ballotStyle.ballot_eml.election_event_presentation
    )

    const handleClose = (value: boolean) => {
        setAuditBallotHelp(false)
        if (value) {
            navigate(
                `/tenant/${tenantId}/event/${eventId}/election/${ballotStyle.election_id}/audit${location.search}`
            )
        }
    }

    const handleCloseCastVoteDialog = (value: boolean) => {
        setConfirmCastVoteModal(false)
        if (value) {
            castBallotAction()
        }
    }

    const handleCloseUnfilledContestsDialog = (value: boolean) => {
        setUnfilledContestsModal(false)
        if (value) {
            castBallotAction()
        }
    }

    // Asked again on every cast: the answer is not kept, so a voter who goes
    // back to edit and returns with unfilled positions confirms them anew.
    const onCast = () => {
        if (unfilledContests.length > 0) {
            setUnfilledContestsModal(true)
        } else if (castVoteConfirmModal && !isFullyAcclaimed) {
            setConfirmCastVoteModal(true)
        } else {
            castBallotAction()
        }
    }

    const storeBallotDataAndReauth = async (ballotData: SessionBallotData) => {
        const errorType = VotingPortalErrorType.UNABLE_TO_CAST_BALLOT
        try {
            // Set a 5-minute expiration for security
            const FIVE_MINUTES = 5 * 60 * 1000
            // Store the data with expiration info
            const owner = {
                tenantId: tenantId ?? "",
                eventId: eventId ?? "",
                userId: authContext.userId,
            }
            const previous = snapshotSessionBallotData()
            let reuse = false
            try {
                const saved = JSON.parse(previous.serialized ?? "null") as SessionBallotData | null
                reuse =
                    !!saved &&
                    saved.ballotId === ballotData.ballotId &&
                    saved.electionId === ballotData.electionId &&
                    saved.ballot === ballotData.ballot &&
                    JSON.stringify(saved.owner) === JSON.stringify(owner) &&
                    Number(previous.expiry) > Date.now() &&
                    Number(previous.expiry) <= Date.now() + FIVE_MINUTES
            } catch {
                /* A malformed request cannot be retried. */
            }
            if (!reuse) {
                sessionStorage.setItem(BALLOT_DATA_KEY, JSON.stringify({...ballotData, owner}))
                sessionStorage.setItem(
                    BALLOT_DATA_EXPIRATION_KEY,
                    (Date.now() + FIVE_MINUTES).toString()
                )
            }
        } catch (e) {
            console.error("Error saving ballotData to sessionStorage", e)
            clearSessionStorageBallotData()
            setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.SESSION_STORAGE_ERROR}`))
            return submit({error: errorType}, {method: "post"})
        }
        try {
            const baseUrl = new URL(window.location.href)
            await reauthWithGold(baseUrl.toString())
            setCasting(false)
        } catch (error) {
            console.error("Re-authentication failed")
            setCasting(false)
            setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.REAUTH_FAILED}`))
        }
    }

    const backNavigateTo = isDeclineToVote
        ? `/tenant/${tenantId}/event/${eventId}/election/${ballotStyle.election_id}/start${location.search}`
        : `/tenant/${tenantId}/event/${eventId}/election/${ballotStyle.election_id}/vote${location.search}`

    const castBallotAction = async () => {
        if (castingRef.current || castRefused) {
            return
        }
        // A fully acclaimed election produces no ballot, so there is nothing
        // to encrypt, hash or cast: the voter goes straight to confirmation.
        if (isFullyAcclaimed) {
            setCasting(true)
            dispatch(completeAcclaimedElection(ballotStyle.election_id))
            return submit(null, {method: "post"})
        }
        const errorType = VotingPortalErrorType.UNABLE_TO_CAST_BALLOT
        setCasting(true)
        if (isGoldenPolicy) {
            // Authentication can fail before leaving this page. Reuse the signed
            // request and its original deadline rather than signing it again.
            const saved = snapshotSessionBallotData()
            try {
                const pending = JSON.parse(saved.serialized ?? "null") as SessionBallotData | null
                const owner = {
                    tenantId: tenantId ?? "",
                    eventId: eventId ?? "",
                    userId: authContext.userId,
                }
                if (
                    pending?.ballotId === ballotId &&
                    pending.electionId === ballotStyle.election_id &&
                    pending.isDemo === isDemo &&
                    JSON.stringify(pending.owner) === JSON.stringify(owner) &&
                    Number(saved.expiry) > Date.now() &&
                    Number(saved.expiry) <= Date.now() + 300_000
                ) {
                    return await storeBallotDataAndReauth(pending)
                }
            } catch {
                /* A malformed request is replaced by the current ballot. */
            }
        }
        if (isDemo || globalSettings.DISABLE_AUTH) {
            if (isGoldenPolicy) {
                // Save contests to session storage and perform reauthentication
                const ballotData: SessionBallotData = {
                    ballotId,
                    auditButtonCfg,
                    electionId: ballotStyle.election_id,
                    isDemo: true,
                    ballot: JSON.stringify("{}"),
                    timestamp: Date.now(), // Add timestamp for expiration check
                }
                return await storeBallotDataAndReauth(ballotData)
            } else {
                addFakeCastVote()
                dispatch(
                    setConfirmationScreenData({
                        electionId: ballotStyle.election_id,
                        confirmationScreenData: {ballotId, isDemo: true, auditButtonCfg},
                    })
                )
                dispatch(clearAuditableBallot(ballotStyle.election_id))
                provideBallotService().forgetVoterSigningKey(ballotStyle.election_id)
                return submit(null, {method: "post"})
            }
        }

        let hashableBallot: IHashableSingleBallot | IHashableMultiBallot | undefined
        try {
            hashableBallot = isMultiContest
                ? toHashableMultiBallot(auditableBallot as IAuditableMultiBallot)
                : toHashableBallot(auditableBallot as IAuditableSingleBallot)
        } catch (error) {
            setCasting(false)
            console.error(error)
            let ballotError = (error as IBallotError) || undefined
            if (ballotError?.error_type) {
                setErrorMsg(t(`reviewScreen.error.${ballotError.error_type}`))
            } else {
                setErrorMsg(
                    t(`reviewScreen.error.${CastBallotsErrorType.TO_HASHABLE_BALLOT_ERROR}`)
                )
            }

            return submit({error: errorType}, {method: "post"})
        }

        // With receipts on, the voter casts by signing the Ballot ID with the
        // key that signed the ballot. A page that no longer holds that key
        // goes back to the ballot, which signs and sends a new one.
        let receivedCast: IReceivedCast | undefined
        const ballotBoxKey = ballotStyle.ballot_eml.ballot_box_key
        if (receivedBallot && ballotBoxKey) {
            try {
                receivedCast = signReceivedCast(receivedBallot, ballotBoxKey)
            } catch (error) {
                console.error("The key that signed the ballot is gone:", error)
                setCasting(false)
                return navigate(backNavigateTo)
            }
        }

        /**
         * For high-security elections (golden policy):
         * 1. Save ballot information to browser session storage
         * 2. Perform secondary authentication ("reauthentication")
         * 3. Submit ballot only after successful verification of voter's identity
         */
        if (isGoldenPolicy) {
            // Save contests to session storage and perform reauthentication
            const ballotData: SessionBallotData = {
                ballotId,
                receivedCast,
                auditButtonCfg,
                electionId: ballotStyle.election_id,
                isDemo,
                ballot: JSON.stringify(hashableBallot),
                timestamp: Date.now(), // Add timestamp for expiration check
                ...(transparency.mode === ETransparencyMode.DISABLED ? {} : {transparency}),
            }
            return await storeBallotDataAndReauth(ballotData)
        }

        const cast = receivedCast
            ? receivedCastOutcome(await tryCastReceivedBallot(receivedCast, setErrorMsg))
            : await tryInsertCastVote(
                  ballotStyle.election_id,
                  ballotId,
                  JSON.stringify(hashableBallot),
                  setErrorMsg,
                  undefined,
                  transparency
              )
        if (!cast.ok) {
            setCasting(false)
            return submit({error: errorType}, {method: "post"})
        }
        return submit(null, {method: "post"})
    }

    return (
        <>
            {auditButtonCfg === EVotingPortalAuditButtonCfg.SHOW ? (
                <AuditBallotHelpDialog
                    auditBallotHelp={auditBallotHelp}
                    handleClose={handleClose}
                />
            ) : null}
            {/* The row itself is `ReviewActions` in `ui-essentials`, so the Election
                Architect's preview draws these three buttons rather than three of its
                own. What stays here is everything that acts: the mutation, the audit
                policy and the dialogs. */}
            <ReviewActions
                withAudit={auditButtonCfg === EVotingPortalAuditButtonCfg.SHOW && !isFullyAcclaimed}
                casting={isCasting}
                isFullyAcclaimed={isFullyAcclaimed}
                backComponent={RouterLink}
                backTo={backNavigateTo}
                onAudit={() => setAuditBallotHelp(true)}
                // A detected hash mismatch refuses the cast, and so does a ballot
                // the ballot box has not received; leaving the handler out is
                // what disables the button.
                onCast={castRefused ? undefined : onCast}
            />
            <UnfilledContestsDialog
                open={isUnfilledContestsModal}
                unfilledContests={unfilledContests}
                handleClose={handleCloseUnfilledContestsDialog}
            />
            <Dialog
                className="confirm-cast-ballot-dialog"
                handleClose={handleCloseCastVoteDialog}
                open={isConfirmCastVoteModal}
                title={t(
                    isBlankBallot
                        ? "reviewScreen.confirmCastBlankBallotDialog.title"
                        : "reviewScreen.confirmCastVoteDialog.title"
                )}
                ok={t(
                    isBlankBallot
                        ? "reviewScreen.confirmCastBlankBallotDialog.ok"
                        : "reviewScreen.confirmCastVoteDialog.ok"
                )}
                cancel={t(
                    isBlankBallot
                        ? "reviewScreen.confirmCastBlankBallotDialog.cancel"
                        : "reviewScreen.confirmCastVoteDialog.cancel"
                )}
                variant="info"
            >
                {stringToHtml(
                    t(
                        isBlankBallot
                            ? "reviewScreen.confirmCastBlankBallotDialog.content"
                            : "reviewScreen.confirmCastVoteDialog.content"
                    )
                )}
            </Dialog>
        </>
    )
}

enum ECastRecovery {
    READY = "ready",
    CASTING = "casting",
    RETRY = "retry",
    EXPIRED = "expired",
}

export const ReviewScreen: React.FC = () => {
    const {electionId} = useParams<{electionId?: string}>()
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionId)))
    // When every contest is acclaimed there is no ballot at all: no
    // encryption, no cast vote and no ballot id to show or audit.
    const isFullyAcclaimed = areAllContestsAcclaimed(ballotStyle?.ballot_eml.contests)
    const location = useLocation()
    const auditableBallot = useAppSelector(selectAuditableBallot(String(electionId)))
    const auditStatus = useAppSelector(selectBallotAuditStatus(String(electionId)))
    const [auditBallotHelp, setAuditBallotHelp] = useState<boolean>(false)
    const [openBallotIdHelp, setOpenBallotIdHelp] = useState(false)
    const [openReviewScreenHelp, setReviewScreenHelp] = useState(false)
    const {
        interpretContestSelection,
        interpretMultiContestSelection,
        decodeAuditableBallot,
        decodeAuditableMultiBallot,
    } = provideBallotService()
    const {t} = useTranslation()
    const backLink = useRootBackLink()
    const navigate = useNavigate()
    const submit = useSubmit()
    const {tenantId, eventId} = useParams<TenantEventType>()
    useEffect(() => {
        if (auditStatus === EBallotAuditStatus.AUDITED)
            navigate(
                `/tenant/${tenantId}/event/${eventId}/election/${electionId}/vote${location.search}`
            )
    }, [auditStatus, tenantId, eventId, electionId, navigate, location.search])
    const [errorMsg, setErrorMsg] = useState<string>()
    const authContext = useContext(AuthContext)
    const [recovery, setRecovery] = useState(ECastRecovery.READY)
    const recoveryAttempt = useRef(0)
    const recoveryMounted = useRef(true)
    const recoveryOwner = JSON.stringify([tenantId, eventId, electionId, authContext.userId])
    const currentRecoveryOwner = useRef(recoveryOwner)
    currentRecoveryOwner.current = recoveryOwner
    useEffect(() => {
        recoveryMounted.current = true
        return () => {
            recoveryMounted.current = false
            recoveryAttempt.current += 1
        }
    }, [])
    const {isGoldUser, reauthWithGold} = authContext
    const {castingRef, isCasting, setCasting} = useCastingState()
    const {globalSettings} = useContext(SettingsContext)
    useEffect(() => {
        recoveryAttempt.current += 1
        setCasting(false)
        setRecovery(ECastRecovery.READY)
        setErrorMsg(undefined)
    }, [recoveryOwner])
    const slates = useBallotStyleSlates(ballotStyle)
    const dispatch = useAppDispatch()
    const addFakeCastVote = useAddFakeCastVote(tenantId, eventId)
    const tryInsertCastVote = useTryInsertCastVote()
    const tryCastReceivedBallot = useCastReceivedBallot()
    const electionFromRedux = useAppSelector(selectElectionById(String(electionId)))
    const electionName = useElectionName(electionId)
    const events = useAppSelector((state) => state.electionEvent)
    const keepsTransparencyLog =
        eventTransparency(
            events[ballotStyle?.election_event_id ?? ""]?.presentation,
            ballotStyle?.ballot_eml?.election_event_presentation
        ).mode !== ETransparencyMode.DISABLED
    const isDeclineToVote = useAppSelector(isDeclineToVoteByElectionId(String(electionId)))
    const {data: dataElections, error: errorElections} = useQuery<GetElectionsQuery>(
        GET_ELECTIONS,
        {
            variables: {
                electionIds: electionId ? [electionId] : [],
            },
            skip: globalSettings.DISABLE_AUTH || electionFromRedux !== undefined, // Skip query if we can get the election from redux (golden user cant)
        }
    )

    useEffect(() => {
        if (!errorElections) return

        const isNetworkError =
            errorElections.name === "NetworkError" ||
            errorElections.message.includes("fetch") ||
            errorElections.message.includes("network")

        if (isNetworkError) {
            setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.NETWORK_ERROR}`))
        } else {
            setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.UNABLE_TO_FETCH_DATA}`))
        }
    }, [errorElections, t])

    const isGoldenPolicy = useMemo(() => {
        return electionFromRedux !== undefined
            ? electionFromRedux?.presentation?.cast_vote_gold_level ===
                  ECastVoteGoldLevelPolicy.GOLD_LEVEL
            : dataElections?.sequent_backend_election?.some(
                  (item) =>
                      item?.id === electionId &&
                      item?.presentation?.cast_vote_gold_level ===
                          ECastVoteGoldLevelPolicy.GOLD_LEVEL
              )
    }, [electionFromRedux, dataElections])

    console.log(electionFromRedux !== undefined)

    const auditButtonCfg =
        ballotStyle?.ballot_eml?.election_presentation?.audit_button_cfg ??
        EVotingPortalAuditButtonCfg.SHOW
    const castVoteConfirmModal =
        ballotStyle?.ballot_eml?.election_presentation?.cast_vote_confirm ?? false

    const isMultiContest =
        auditableBallot?.config.election_event_presentation?.contest_encryption_policy ==
        EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS
    const hashableBallot = auditableBallot
        ? isMultiContest
            ? hashMultiBallot(auditableBallot as IAuditableMultiBallot)
            : hashBallot(auditableBallot as IAuditableSingleBallot)
        : undefined

    const ballotId = useMemo(() => {
        return auditableBallot && hashableBallot ? hashableBallot : undefined
    }, [auditableBallot, hashableBallot])

    const selectionState = useAppSelector(
        selectBallotSelectionByElectionId(ballotStyle?.election_id ?? "")
    )

    const isBlankBallot = Boolean(
        selectionState?.length && selectionState.every((contest) => contest.is_blank_ballot)
    )

    // The review shows the selections in the store and casts the encrypted
    // ballot. They differ when the selections changed after encrypting, for
    // example through the browser history.
    const hasStaleBallot = useMemo(() => {
        if (!auditableBallot || !selectionState) {
            return false
        }
        try {
            const encryptedSelection = isMultiContest
                ? decodeAuditableMultiBallot(auditableBallot as IAuditableMultiBallot)
                : decodeAuditableBallot(auditableBallot as IAuditableSingleBallot)
            return (
                !!encryptedSelection && !isSameBallotSelection(encryptedSelection, selectionState)
            )
        } catch (error) {
            console.log(`Error decoding the ballot under review: ${error}`)
            return false
        }
    }, [auditableBallot, selectionState, isMultiContest])

    const votePath = `/tenant/${tenantId}/event/${eventId}/election/${electionId}/vote${location.search}`

    useEffect(() => {
        if (hasStaleBallot) {
            navigate(votePath, {replace: true})
        }
    }, [hasStaleBallot, navigate, votePath])

    const errorSelectionState = useMemo(() => {
        if (!selectionState || !ballotStyle || isFullyAcclaimed || hasStaleBallot) {
            return []
        }
        return isMultiContest
            ? interpretMultiContestSelection(selectionState, ballotStyle.ballot_eml)
            : interpretContestSelection(selectionState, ballotStyle.ballot_eml)
    }, [selectionState, isMultiContest, ballotStyle?.ballot_eml, hasStaleBallot])

    const reviewSummary = useMemo(
        () =>
            ballotStyle && slates.resolved?.slates.length && !isDeclineToVote && !isFullyAcclaimed
                ? getBallotReviewSummary(
                      ballotStyle.ballot_eml.contests,
                      slates.resolved,
                      selectionState
                  )
                : undefined,
        [ballotStyle, slates.resolved, selectionState, isDeclineToVote, isFullyAcclaimed]
    )

    const hasInconsistentHash = Boolean(
        ballotId && auditableBallot?.ballot_hash && ballotId !== auditableBallot.ballot_hash
    )
    const hashErrorMsg = hasInconsistentHash
        ? t(
              `reviewScreen.error.${CastBallotsErrorType.INCONSISTENT_HASH}`,
              escapeTranslationValues({
                  ballotId,
                  auditableBallotHash: auditableBallot?.ballot_hash ?? "",
              })
          )
        : undefined
    const receipt = useReceiveBallot({
        ballotStyle,
        auditableBallot,
        ballotHash: ballotId,
        isMultiContest,
        skip:
            !!ballotStyle?.ballot_eml?.public_key?.is_demo ||
            globalSettings.DISABLE_AUTH ||
            isFullyAcclaimed ||
            hasInconsistentHash,
    })
    const isReceiptRequired = receipt.status !== EReceiveBallotStatus.NOT_REQUIRED
    const isAwaitingReceipt = isReceiptRequired && receipt.status !== EReceiveBallotStatus.RECEIVED
    // With receipts on, the Ballot ID is the ballot box's: there is none to
    // show until the ballot box has stored and signed the ballot.
    const shownBallotId = isReceiptRequired ? receipt.ballotId : ballotId || ""
    const displayedErrorMsg = hashErrorMsg ?? receipt.errorMsg ?? errorMsg

    const handleCloseDialogAuditHelp = (value: boolean) => {
        setAuditBallotHelp(false)
        if (value) {
            navigate(
                `/tenant/${tenantId}/event/${eventId}/election/${ballotStyle?.election_id}/audit${location.search}`
            )
        }
    }

    // Dismiss only. Auditing is reached from AuditButton, which confirms through
    // AuditBallotHelpDialog first; this dialog used to navigate there too, which
    // is the duplicate audit trigger reported in META-12776. Matches how
    // AuditScreen renders the same dialog.
    function handleCloseDialogIdHelp() {
        setOpenBallotIdHelp(false)
    }

    const getBallotDataFromSessionStorage = (snapshot: SessionBallotSnapshot) => {
        const owner = {tenantId: tenantId ?? "", eventId: eventId ?? "", userId: authContext.userId}
        const fail = (
            error: CastBallotsErrorType,
            expired = false,
            locator?: SessionBallotLocator
        ) => {
            clearSessionStorageBallotData(snapshot, locator)
            setErrorMsg(t(`reviewScreen.error.${error}`))
            setRecovery(expired ? ECastRecovery.EXPIRED : ECastRecovery.RETRY)
            return undefined
        }
        if (!snapshot.serialized && !snapshot.expiry) {
            const locator = readSessionBallotLocator(owner, electionId ?? "")
            if (locator) return fail(CastBallotsErrorType.SESSION_EXPIRED, true, locator)
        }
        if (!snapshot.serialized || !snapshot.expiry)
            return fail(CastBallotsErrorType.SESSION_STORAGE_ERROR)
        const expiresAt = Number(snapshot.expiry)
        if (!Number.isFinite(expiresAt) || expiresAt > Date.now() + 5 * 60 * 1000)
            return fail(CastBallotsErrorType.NOT_VALID_BALLOT_DATA_ERROR)
        try {
            const data = JSON.parse(snapshot.serialized) as SessionBallotData
            if (
                !data ||
                typeof data.ballotId !== "string" ||
                !data.ballotId ||
                data.electionId !== electionId ||
                typeof data.ballot !== "string" ||
                !data.ballot
            )
                return fail(CastBallotsErrorType.NOT_VALID_BALLOT_DATA_ERROR)
            if (
                !data.owner ||
                data.owner.tenantId !== tenantId ||
                data.owner.eventId !== eventId ||
                data.owner.userId !== authContext.userId
            )
                return fail(CastBallotsErrorType.NOT_VALID_BALLOT_DATA_ERROR)
            const receipt = data.receivedCast?.receivedBallot
            if (
                data.receivedCast &&
                (!receipt ||
                    receipt.tenant_id !== tenantId ||
                    receipt.election_event_id !== eventId ||
                    receipt.election_id !== electionId ||
                    receipt.ballot_hash !== data.ballotId ||
                    !data.receivedCast.castSignature ||
                    !/^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$/.test(receipt.ballot_id))
            )
                return fail(CastBallotsErrorType.NOT_VALID_BALLOT_DATA_ERROR)
            if (Date.now() >= expiresAt)
                return fail(CastBallotsErrorType.SESSION_EXPIRED, true, {
                    owner,
                    electionId: data.electionId,
                    ballotId: receipt?.ballot_id ?? data.ballotId,
                })
            return data
        } catch {
            return fail(CastBallotsErrorType.PARSE_BALLOT_DATA_ERROR)
        }
    }

    const goldenUserCastBallotAction = async () => {
        if (
            castingRef.current ||
            hasInconsistentHash ||
            (!globalSettings.DISABLE_AUTH && !authContext.userId)
        )
            return
        const snapshot = snapshotSessionBallotData()
        const ballotData = getBallotDataFromSessionStorage(snapshot)
        if (!ballotData) return
        const attempt = ++recoveryAttempt.current
        const owner = recoveryOwner
        const isCurrent = () =>
            recoveryMounted.current &&
            recoveryAttempt.current === attempt &&
            currentRecoveryOwner.current === owner &&
            isCurrentSessionBallotData(snapshot)
        setCasting(true)
        setRecovery(ECastRecovery.CASTING)
        setErrorMsg(undefined)
        if (!isGoldUser()) {
            try {
                await reauthWithGold(window.location.href)
                if (isCurrent()) {
                    setRecovery(ECastRecovery.RETRY)
                    setCasting(false)
                }
            } catch {
                if (isCurrent()) {
                    setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.REAUTH_FAILED}`))
                    setRecovery(ECastRecovery.RETRY)
                    setCasting(false)
                }
            }
            return
        }
        const setCurrentError = (message: string) => {
            if (isCurrent()) setErrorMsg(message)
        }
        const savedTransparency = ballotData.transparency && asTransparency(ballotData.transparency)
        const cast: TCastOutcome = ballotData.isDemo
            ? {ok: true, vitrine: null}
            : ballotData.receivedCast
              ? receivedCastOutcome(
                    await tryCastReceivedBallot(ballotData.receivedCast, setCurrentError, isCurrent)
                )
              : await tryInsertCastVote(
                    ballotData.electionId,
                    ballotData.ballotId,
                    ballotData.ballot,
                    setCurrentError,
                    isCurrent,
                    savedTransparency
                )
        if (!isCurrent()) return
        if (!cast.ok) {
            setCasting(false)
            setRecovery(ECastRecovery.RETRY)
            return
        }
        if (ballotData.isDemo) {
            addFakeCastVote()
            dispatch(clearAuditableBallot(ballotData.electionId))
            provideBallotService().forgetVoterSigningKey(ballotData.electionId)
        }
        // Restore confirmation data after the reauthentication reload.
        const recoveredVitrine = confirmationVitrine(savedTransparency, cast.vitrine)
        dispatch(
            setConfirmationScreenData({
                electionId: ballotData.electionId,
                confirmationScreenData: {
                    ballotId:
                        ballotData.receivedCast?.receivedBallot.ballot_id ?? ballotData.ballotId,
                    isDemo: ballotData.isDemo,
                    auditButtonCfg: ballotData.auditButtonCfg,
                    ...(recoveredVitrine ? {vitrine: recoveredVitrine} : {}),
                },
            })
        )
        clearSessionStorageBallotData(snapshot)
        return submit(null, {method: "post"})
    }

    const needsRecovery =
        (!ballotStyle || !auditableBallot || !selectionState) && isGoldenPolicy && !isFullyAcclaimed
    useEffect(() => {
        if (needsRecovery && recovery === ECastRecovery.READY && !castingRef.current)
            void goldenUserCastBallotAction()
    }, [needsRecovery, recovery, recoveryOwner])

    if (needsRecovery && !hasInconsistentHash) {
        const saved = snapshotSessionBallotData()
        const canRetry = recovery === ECastRecovery.RETRY && !!saved.serialized && !!saved.expiry
        let savedBallotId =
            readSessionBallotLocator(
                {tenantId: tenantId ?? "", eventId: eventId ?? "", userId: authContext.userId},
                electionId ?? ""
            )?.ballotId ?? ""
        try {
            const data = JSON.parse(saved.serialized ?? "null") as SessionBallotData | null
            if (
                data &&
                data.owner &&
                data.owner.tenantId === tenantId &&
                data.owner.eventId === eventId &&
                data.owner.userId === authContext.userId &&
                data.electionId === electionId
            )
                savedBallotId =
                    data.receivedCast?.receivedBallot.ballot_id ?? data.ballotId ?? savedBallotId
        } catch {
            /* Invalid data is already rejected by recovery. */
        }
        return (
            <Box className="review-cast-recovery" sx={{margin: "auto 0", textAlign: "center"}}>
                {recovery === ECastRecovery.CASTING || recovery === ECastRecovery.READY ? (
                    <CircularProgress
                        className="review-cast-recovery-loading"
                        aria-label={t("a11y.loading")}
                    />
                ) : (
                    <>
                        <WarnBox
                            className="review-cast-recovery-error"
                            variant="error"
                            announcement={EWarnBoxAnnouncement.ASSERTIVE}
                        >
                            {errorMsg && stringToHtml(errorMsg)}
                            {t("reviewScreen.castRecovery.uncertain")}
                        </WarnBox>
                        {canRetry && (
                            <StyledButton
                                className="review-cast-recovery-retry-button"
                                onClick={() => void goldenUserCastBallotAction()}
                            >
                                {t("ballotPrivacy.retry")}
                            </StyledButton>
                        )}
                        <StyledButton
                            className="review-cast-recovery-back-button"
                            onClick={() => {
                                recoveryAttempt.current += 1
                                clearSessionStorageBallotData(saved)
                                navigate(backLink)
                            }}
                        >
                            {t("reviewScreen.backButton")}
                        </StyledButton>
                        <StyledButton
                            className="review-cast-recovery-check-button"
                            component={RouterLink}
                            to={`/tenant/${tenantId}/event/${eventId}/election/${electionId}/ballot-locator/${encodeURIComponent(savedBallotId)}${location.search}`}
                        >
                            {t(
                                savedBallotId
                                    ? "reviewScreen.castRecovery.checkBallot"
                                    : "ballotLocator.locate"
                            )}
                        </StyledButton>
                    </>
                )}
            </Box>
        )
    }

    if (hasStaleBallot) {
        return <CircularProgress className="review-progress" aria-label={t("a11y.loading")} />
    }

    if (!ballotStyle || (!auditableBallot && !isFullyAcclaimed)) {
        return errorMsg ? (
            <Box className="review-error-screen" sx={{margin: "auto 0"}}>
                {/* The only message on the screen, and it blocks the voter from
                    casting, so it interrupts rather than waiting for a pause. */}
                <WarnBox
                    className="cast-ballot-error"
                    variant="error"
                    announcement={EWarnBoxAnnouncement.ASSERTIVE}
                >
                    {stringToHtml(errorMsg)}
                </WarnBox>
                <Box
                    className="review-error-actions"
                    sx={{
                        display: "flex",
                        justifyContent: "center",
                        width: "100%",
                        marginTop: "16px",
                    }}
                >
                    <StyledButton
                        className="back-button"
                        component={RouterLink}
                        to={backLink}
                        sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
                    >
                        <Icon className="back-button-icon" icon={faAngleLeft} size="sm" />
                        <Box className="back-button-label">{t("reviewScreen.backButton")}</Box>
                    </StyledButton>
                </Box>
            </Box>
        ) : (
            <CircularProgress className="review-progress" aria-label={t("a11y.loading")} />
        )
    }

    const contests = orderContestsForReview(ballotStyle)
    const defaultLanguageCode =
        ballotStyle.ballot_eml.election_presentation?.language_conf?.default_language_code ??
        ballotStyle.ballot_eml.election_event_presentation?.language_conf?.default_language_code
    // A voter who declined to vote already confirmed leaving the ballot empty.
    const unfilledContests = isDeclineToVote
        ? []
        : getUnfilledContests(contests, errorSelectionState)

    return (
        // The arrangement is `ReviewLayout`, in `ui-essentials`, so that the
        // Election Architect's ballot preview shows this screen rather than a
        // copy of it that drifts. What stays here is everything that needs the
        // store, the mutation or this screen's own state: the dialogs, the
        // breadcrumb that knows whether an election list counts as a step, and
        // the actions that cast.
        <BallotSlatesProvider slates={slates.config}>
            <ReviewLayout
                ballotId={
                    auditButtonCfg === EVotingPortalAuditButtonCfg.NOT_SHOW || isFullyAcclaimed
                        ? undefined
                        : shownBallotId
                }
                onBallotIdHelp={() => setOpenBallotIdHelp(true)}
                trackingIdBox={
                    auditButtonCfg !== EVotingPortalAuditButtonCfg.NOT_SHOW &&
                    !isFullyAcclaimed &&
                    keepsTransparencyLog &&
                    shownBallotId
                        ? (
                              <ReviewBallotId
                                  ballotId={shownBallotId}
                                  electionName={electionName}
                                  helpLabel={t("reviewScreen.ballotIdHelpDialog.title")}
                                  onHelpClick={() => setOpenBallotIdHelp(true)}
                              />
                          )
                        : undefined
                }
                steps={<Stepper selected={2} />}
                onTitleHelp={() => setReviewScreenHelp(true)}
                error={displayedErrorMsg ? stringToHtml(displayedErrorMsg) : undefined}
                // The layout says `reviewScreen.*` for itself now. What it cannot know is
                // whether this event offers *Audit ballot*, which is what chooses between the
                // two descriptions — one of them mentions a button the other event lacks.
                withAudit={
                    auditButtonCfg !== EVotingPortalAuditButtonCfg.NOT_SHOW &&
                    auditButtonCfg !== EVotingPortalAuditButtonCfg.SHOW_IN_HELP
                }
                isFullyAcclaimed={isFullyAcclaimed}
                ballotStyle={ballotStyle}
                contests={contests}
                errorSelectionState={errorSelectionState}
                isDeclineToVote={isDeclineToVote}
                isBlankBallot={isBlankBallot}
                summary={
                    reviewSummary ? (
                        <ReviewSelectionSummary
                            summary={reviewSummary}
                            defaultLanguage={defaultLanguageCode}
                        />
                    ) : undefined
                }
                renderContestFooter={(contest) => {
                    const count = reviewSummary?.contests[contest.id]
                    return count ? (
                        <ReviewContestFooter contest={contest} count={count} to={votePath} />
                    ) : null
                }}
                actions={
                    isCasting ? undefined : (
                        <ActionButtons
                            ballotStyle={ballotStyle}
                            auditableBallot={auditableBallot}
                            auditButtonCfg={auditButtonCfg}
                            castVoteConfirmModal={castVoteConfirmModal}
                            ballotId={ballotId ?? ""}
                            setErrorMsg={setErrorMsg}
                            hasInconsistentHash={hasInconsistentHash}
                            isAwaitingReceipt={isAwaitingReceipt}
                            receivedBallotId={receipt.ballotId}
                            isGoldenPolicy={isGoldenPolicy ?? false}
                            isMultiContest={isMultiContest}
                            isDeclineToVote={isDeclineToVote}
                            isBlankBallot={isBlankBallot}
                            isFullyAcclaimed={isFullyAcclaimed}
                            unfilledContests={unfilledContests}
                        />
                    )
                }
            >
                <BallotIdHelpDialog
                    className="review-ballot-id-help-dialog"
                    handleClose={handleCloseDialogIdHelp}
                    open={openBallotIdHelp}
                    title={t("reviewScreen.ballotIdHelpDialog.title")}
                    maxWidth="md"
                    middleActions={
                        auditButtonCfg === EVotingPortalAuditButtonCfg.SHOW_IN_HELP
                            ? [
                                  <AuditButton
                                      key={"audit-button"}
                                      onClick={() => {
                                          setOpenBallotIdHelp(false)
                                          setAuditBallotHelp(true)
                                      }}
                                  />,
                              ]
                            : []
                    }
                    cancel={t("reviewScreen.ballotIdHelpDialog.cancel")}
                    variant="info"
                >
                    {stringToHtml(t("reviewScreen.ballotIdHelpDialog.content"))}
                </BallotIdHelpDialog>
                {auditButtonCfg === EVotingPortalAuditButtonCfg.SHOW_IN_HELP &&
                !isFullyAcclaimed ? (
                    <AuditBallotHelpDialog
                        auditBallotHelp={auditBallotHelp}
                        handleClose={handleCloseDialogAuditHelp}
                    />
                ) : null}
                {/* The title's own help dialog. It sat inside the heading before,
                which made no difference to a reader — MUI renders a dialog into
                a portal wherever it is declared — and made the heading harder to
                lift out. */}
                <Dialog
                    className="screen-help-dialog review-help-dialog"
                    handleClose={() => setReviewScreenHelp(false)}
                    open={openReviewScreenHelp}
                    title={t(
                        isFullyAcclaimed
                            ? "reviewScreen.acclamation.helpDialog.title"
                            : "reviewScreen.reviewScreenHelpDialog.title"
                    )}
                    ok={t(
                        isFullyAcclaimed
                            ? "reviewScreen.acclamation.helpDialog.ok"
                            : "reviewScreen.reviewScreenHelpDialog.ok"
                    )}
                    variant="info"
                >
                    {stringToHtml(
                        t(
                            isFullyAcclaimed
                                ? "reviewScreen.acclamation.helpDialog.content"
                                : "reviewScreen.reviewScreenHelpDialog.content"
                        )
                    )}
                </Dialog>
            </ReviewLayout>
        </BallotSlatesProvider>
    )
}

export default ReviewScreen

export async function action({request}: {request: Request}) {
    const data = await request.formData()
    const error = data.get("error")

    if (!error) {
        return redirect("../confirmation")
    }

    return null
}
