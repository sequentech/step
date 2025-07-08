// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState, useContext, useMemo, useRef} from "react"
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
import {
    PageLimit,
    Icon,
    IconButton,
    theme,
    BallotHash,
    Dialog,
    WarnBox,
} from "@sequentech/ui-essentials"
import {
    stringToHtml,
    EVotingStatus,
    IElectionEventStatus,
    IAuditableBallot,
    EVotingPortalAuditButtonCfg,
    IGraphQLActionError,
    IAuditableSingleBallot,
    IAuditableMultiBallot,
    ECastVoteGoldLevelPolicy,
    EElectionEventContestEncryptionPolicy,
    sortContestList,
    hashBallot,
    hashMultiBallot,
} from "@sequentech/ui-core"
import {styled} from "@mui/material/styles"
import Typography from "@mui/material/Typography"
import {
    faCircleQuestion,
    faAngleLeft,
    faAngleRight,
    faFire,
} from "@fortawesome/free-solid-svg-icons"
import {useTranslation} from "react-i18next"
import Button from "@mui/material/Button"
import {selectAuditableBallot} from "../store/auditableBallots/auditableBallotsSlice"
import {Question} from "../components/Question/Question"
import {useMutation, useQuery} from "@apollo/client"
import {INSERT_CAST_VOTE} from "../queries/InsertCastVote"
import {GetElectionEventQuery, InsertCastVoteMutation, GetElectionsQuery} from "../gql/graphql"
import {GET_ELECTIONS} from "../queries/GetElections"
import {GET_ELECTION_EVENT} from "../queries/GetElectionEvent"
import {provideBallotService} from "../services/BallotService"
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
import {SettingsContext} from "../providers/SettingsContextProvider"
import {AuthContext} from "../providers/AuthContextProvider"
import {useGetOne} from "react-admin"

// Extended SessionBallotData interface with timestamp for expiration
interface SessionBallotData {
    ballotId: string
    electionId: string
    isDemo: boolean
    ballot: string
    timestamp?: number
}

const StyledLink = styled(RouterLink)`
    margin: auto 0;
    text-decoration: none;
`

const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
`

const ActionsContainer = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    gap: 2px;
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

const StyledIcon = styled(Icon)`
    min-width: 14px;
    padding: 5px;
`

const StyledCircularProgress = styled(CircularProgress)`
    width: 14px !important;
    height: 14px !important;
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
            <Icon icon={faFire} size="sm" />
            <Box>{t("reviewScreen.auditButton")}</Box>
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
interface ActionButtonProps {
    ballotStyle: IBallotStyle
    auditableBallot: IAuditableBallot
    auditButtonCfg: EVotingPortalAuditButtonCfg
    castVoteConfirmModal: boolean
    ballotId: string
    setErrorMsg: (msg: CastBallotsErrorType) => void
}

interface LoadingOrCastButtonProps {
    onClick: () => void
    className?: string
    isCastingBallot: boolean
}

const LoadingOrCastButton: React.FC<LoadingOrCastButtonProps> = ({
    onClick,
    isCastingBallot,
    className,
}) => {
    const {t} = useTranslation()

    return (
        <StyledButton
            className={className}
            sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
            disabled={isCastingBallot}
            onClick={onClick}
        >
            <Box>{t("reviewScreen.castBallotButton")}</Box>
            {isCastingBallot ? (
                <StyledCircularProgress color="inherit" />
            ) : (
                <StyledIcon icon={faAngleRight} size="sm" />
            )}
        </StyledButton>
    )
}

const removeBallotDataFromSessionStorage = () => {
    sessionStorage.removeItem("ballotData")
    sessionStorage.removeItem("ballotDataExpiration")
}

const ActionButtons: React.FC<ActionButtonProps> = ({
    ballotStyle,
    auditableBallot,
    auditButtonCfg,
    castVoteConfirmModal,
    ballotId,
    setErrorMsg,
}) => {
    const dispatch = useAppDispatch()
    const [insertCastVote] = useMutation<InsertCastVoteMutation>(INSERT_CAST_VOTE)
    const {t} = useTranslation()
    const navigate = useNavigate()
    const location = useLocation()
    const [auditBallotHelp, setAuditBallotHelp] = useState<boolean>(false)
    const isCastingBallot = useRef<boolean>(false)
    const [isConfirmCastVoteModal, setConfirmCastVoteModal] = React.useState<boolean>(false)
    const {tenantId, eventId} = useParams<TenantEventType>()
    const {toHashableBallot, toHashableMultiBallot} = provideBallotService()
    const submit = useSubmit()
    const isDemo = !!ballotStyle?.ballot_eml?.public_key?.is_demo
    const {globalSettings} = useContext(SettingsContext)
    const authContext = useContext(AuthContext)
    const {isGoldUser, reauthWithGold} = authContext

    const {refetch: refetchElectionEvent} = useQuery<GetElectionEventQuery>(GET_ELECTION_EVENT, {
        variables: {
            electionEventId: eventId,
            tenantId,
        },
        skip: globalSettings.DISABLE_AUTH, // Skip query if in demo mode
        onError: (error) => {
            if (error.networkError) {
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.NETWORK_ERROR}`))
            } else {
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.UNABLE_TO_FETCH_DATA}`))
            }
        },
    })

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

    const fakeCastVote = (): ICastVote => ({
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
    })

    const castBallotAction = async () => {
        const isGoldenPolicy =
            ballotStyle?.ballot_eml.election_presentation?.cast_vote_gold_level ===
            ECastVoteGoldLevelPolicy.GOLD_LEVEL
        const errorType = VotingPortalErrorType.UNABLE_TO_CAST_BALLOT

        const isMultiContest =
            auditableBallot?.config.election_event_presentation?.contest_encryption_policy ==
            EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS

        const hashableBallot = isMultiContest
            ? toHashableMultiBallot(auditableBallot as IAuditableMultiBallot)
            : toHashableBallot(auditableBallot as IAuditableSingleBallot)

        // Handle demo mode
        if (isDemo || globalSettings.DISABLE_AUTH) {
            if (isGoldenPolicy) {
                try {
                    // Save contests to session storage with timestamp for expiration
                    const ballotData: SessionBallotData = {
                        ballotId,
                        electionId: ballotStyle.election_id,
                        isDemo: true,
                        ballot: JSON.stringify("{}"),
                        timestamp: Date.now(), // Add timestamp for expiration check
                    }
                    sessionStorage.setItem("ballotData", JSON.stringify(ballotData))

                    const baseUrl = new URL(window.location.href)
                    await reauthWithGold(baseUrl.toString())
                    return submit(null, {method: "post"})
                } catch (error) {
                    // Clean up session storage on error
                    removeBallotDataFromSessionStorage()
                    console.error("Re-authentication failed:", error)
                    return submit({error: errorType}, {method: "post"})
                }
            }
            const newCastVote = fakeCastVote()
            dispatch(addCastVotes([newCastVote]))
            return submit(null, {method: "post"})
        }

        isCastingBallot.current = true

        try {
            const {data} = await refetchElectionEvent()

            // Check if election events are available
            if (!(data?.sequent_backend_election_event?.length > 0)) {
                isCastingBallot.current = false
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.LOAD_ELECTION_EVENT}`))
                return submit({error: errorType}, {method: "post"})
            }

            const record = data?.sequent_backend_election_event?.[0]
            const eventStatus = record?.status as IElectionEventStatus | undefined

            // Check if election event is open for voting
            if (eventStatus?.voting_status !== EVotingStatus.OPEN) {
                isCastingBallot.current = false
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.ELECTION_EVENT_NOT_OPEN}`))
                return submit({error: errorType.toString()}, {method: "post"})
            }

            /**
             * For high-security elections (golden policy):
             * 1. Save ballot information to browser session storage
             * 2. Perform secondary authentication ("reauthentication")
             * 3. Submit ballot only after successful verification of voter's identity
             */
            if (isGoldenPolicy) {
                try {
                    // Save contests to session storage with timestamp for expiration
                    const ballotData: SessionBallotData = {
                        ballotId,
                        electionId: ballotStyle.election_id,
                        isDemo,
                        ballot: JSON.stringify(hashableBallot),
                        timestamp: Date.now(), // Add timestamp for expiration check
                    }

                    // Set a 5-minute expiration for security
                    const FIVE_MINUTES = 5 * 60 * 1000

                    // Store the data with expiration info
                    sessionStorage.setItem("ballotData", JSON.stringify(ballotData))
                    sessionStorage.setItem(
                        "ballotDataExpiration",
                        (Date.now() + FIVE_MINUTES).toString()
                    )

                    const baseUrl = new URL(window.location.href)
                    await reauthWithGold(baseUrl.toString())
                    return submit(null, {method: "post"})
                } catch (error) {
                    // Clean up session storage on error
                    removeBallotDataFromSessionStorage()
                    console.error("Re-authentication failed:", error)
                    setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.REAUTH_FAILED}`))
                    return submit({error: errorType}, {method: "post"})
                }
            }

            // Standard vote casting flow
            let result = await insertCastVote({
                variables: {
                    electionId: ballotStyle.election_id,
                    ballotId,
                    content: JSON.stringify(hashableBallot),
                },
            })

            if (result.errors) {
                isCastingBallot.current = false
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.CAST_VOTE}`))
                return submit({error: errorType}, {method: "post"})
            }

            let newCastVote = result.data?.insert_cast_vote

            if (newCastVote) {
                dispatch(addCastVotes([newCastVote]))
            }

            return submit(null, {method: "post"})
        } catch (error) {
            isCastingBallot.current = false
            let castError = error as IGraphQLActionError
            if (castError?.graphQLErrors?.[0]?.extensions?.code) {
                let errorCode = castError?.graphQLErrors?.[0]?.extensions?.code
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.CAST_VOTE}_${errorCode}`))
            } else {
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.CAST_VOTE}`))
            }
            return submit({error: errorType}, {method: "post"})
        }
    }

    return (
        <Box sx={{marginBottom: "10px", marginTop: "10px"}}>
            {auditButtonCfg === EVotingPortalAuditButtonCfg.SHOW ? (
                <AuditBallotHelpDialog
                    auditBallotHelp={auditBallotHelp}
                    handleClose={handleClose}
                />
            ) : null}
            <ActionsContainer className="actions-container">
                <StyledLink
                    to={`/tenant/${tenantId}/event/${eventId}/election/${ballotStyle.election_id}/vote${location.search}`}
                    sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
                >
                    <StyledButton sx={{width: {xs: "100%", sm: "200px"}}}>
                        <Icon icon={faAngleLeft} size="sm" />
                        <Box>{t("reviewScreen.backButton")}</Box>
                    </StyledButton>
                </StyledLink>
                {auditButtonCfg === EVotingPortalAuditButtonCfg.SHOW ? (
                    <AuditButton onClick={() => setAuditBallotHelp(true)} />
                ) : null}
                <LoadingOrCastButton
                    className="cast-ballot-button"
                    isCastingBallot={isCastingBallot.current}
                    onClick={() =>
                        castVoteConfirmModal ? setConfirmCastVoteModal(true) : castBallotAction()
                    }
                />
            </ActionsContainer>
            <Dialog
                handleClose={handleCloseCastVoteDialog}
                open={isConfirmCastVoteModal}
                title={t("reviewScreen.confirmCastVoteDialog.title")}
                ok={t("reviewScreen.confirmCastVoteDialog.ok")}
                cancel={t("reviewScreen.confirmCastVoteDialog.cancel")}
                variant="info"
            >
                {stringToHtml(t("reviewScreen.confirmCastVoteDialog.content"))}
            </Dialog>
        </Box>
    )
}

export const ReviewScreen: React.FC = () => {
    const location = useLocation()
    const {t} = useTranslation()
    const navigate = useNavigate()
    const submit = useSubmit()
    const dispatch = useAppDispatch()
    const backLink = useRootBackLink()

    const {isGoldUser} = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)

    const {electionId} = useParams<{electionId?: string}>()
    const {tenantId, eventId} = useParams<TenantEventType>()

    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionId)))
    const auditableBallot = useAppSelector(selectAuditableBallot(String(electionId)))
    const selectionState = useAppSelector(
        selectBallotSelectionByElectionId(ballotStyle?.election_id ?? "")
    )

    const [auditBallotHelp, setAuditBallotHelp] = useState<boolean>(false)
    const [openBallotIdHelp, setOpenBallotIdHelp] = useState(false)
    const [openReviewScreenHelp, setReviewScreenHelp] = useState(false)
    const [errorMsg, setErrorMsg] = useState<CastBallotsErrorType>()
    const isCastingBallot = useRef<boolean>(false)

    const {interpretContestSelection, interpretMultiContestSelection} = provideBallotService()

    const [insertCastVote] = useMutation<InsertCastVoteMutation>(INSERT_CAST_VOTE)
    const {refetch: refetchElectionEvent} = useQuery<GetElectionEventQuery>(GET_ELECTION_EVENT, {
        variables: {
            electionEventId: eventId,
            tenantId,
        },
        skip: globalSettings.DISABLE_AUTH, // Skip query if in demo mode
        onError: (error) => {
            if (error.networkError) {
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.NETWORK_ERROR}`))
            } else {
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.UNABLE_TO_FETCH_DATA}`))
            }
        },
    })
    const {data: dataElections} = useQuery<GetElectionsQuery>(GET_ELECTIONS, {
        variables: {
            electionIds: electionId ? [electionId] : [],
        },
        skip: globalSettings.DISABLE_AUTH, // Skip query if in demo mode
    })

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

    const fakeCastVote = (): ICastVote => ({
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
    })

    const isGoldenPolicy = dataElections?.sequent_backend_election.some(
        (item) => item.presentation?.cast_vote_gold_level === ECastVoteGoldLevelPolicy.GOLD_LEVEL
    )

    const auditButtonCfg =
        ballotStyle?.ballot_eml?.election_presentation?.audit_button_cfg ??
        EVotingPortalAuditButtonCfg.SHOW
    const castVoteConfirmModal =
        ballotStyle?.ballot_eml?.election_presentation?.cast_vote_confirm ?? false

    const errorSelectionState = useMemo(() => {
        if (!selectionState || !ballotStyle) {
            return []
        }
        return isMultiContest
            ? interpretMultiContestSelection(selectionState, ballotStyle.ballot_eml)
            : interpretContestSelection(selectionState, ballotStyle.ballot_eml)
    }, [selectionState, isMultiContest, ballotStyle?.ballot_eml])

    if (ballotId && auditableBallot?.ballot_hash && ballotId !== auditableBallot?.ballot_hash) {
        setErrorMsg(
            t("errors.encoding.writeInCharsExceeded", {
                ballotId,
                auditableBallotHash: auditableBallot.ballot_hash,
            })
        )
    }

    const handleCloseDialogAuditHelp = (value: boolean) => {
        setAuditBallotHelp(false)
        if (value) {
            navigate(
                `/tenant/${tenantId}/event/${eventId}/election/${ballotStyle?.election_id}/audit${location.search}`
            )
        }
    }

    function handleCloseDialogIdHelp(val: boolean) {
        setOpenBallotIdHelp(false)

        if (val) {
            if (ballotStyle && tenantId && eventId) {
                navigate(
                    `/tenant/${tenantId}/event/${eventId}/election/${ballotStyle.election_id}/audit`
                )
            } else {
                navigate(`/tenant/${tenantId}/event/${eventId}/election-chooser`)
            }
        }
    }

    const automaticCastBallot = async () => {
        const errorType = VotingPortalErrorType.UNABLE_TO_CAST_BALLOT
        let ballotData: SessionBallotData | undefined

        try {
            // Check if ballot data has expired
            const expirationTime = parseInt(
                sessionStorage.getItem("ballotDataExpiration") || "0",
                10
            )

            console.log("aa expirationTime", expirationTime)
            console.log("aa Date.now()", Date.now())

            if (expirationTime && Date.now() > expirationTime) {
                // Data has expired, clean up and return error
                removeBallotDataFromSessionStorage()
                isCastingBallot.current = false
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.SESSION_EXPIRED}`))
                return submit({error: errorType}, {method: "post"})
            }

            const storedData = sessionStorage.getItem("ballotData")
            if (!storedData) {
                isCastingBallot.current = false
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.UNABLE_TO_FETCH_DATA}`))
                return submit({error: errorType}, {method: "post"})
            }

            ballotData = JSON.parse(storedData) as SessionBallotData

            if (
                !ballotData ||
                !ballotData.ballotId ||
                !ballotData.electionId ||
                !ballotData.ballot
            ) {
                removeBallotDataFromSessionStorage()
                isCastingBallot.current = false
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.UNABLE_TO_FETCH_DATA}`))
                return submit({error: errorType}, {method: "post"})
            }
        } catch (error) {
            // Handle JSON parsing errors
            removeBallotDataFromSessionStorage()
            isCastingBallot.current = false
            setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.UNABLE_TO_FETCH_DATA}`))
            return submit({error: errorType}, {method: "post"})
        }

        if (ballotData?.isDemo) {
            const newCastVote = fakeCastVote()
            dispatch(addCastVotes([newCastVote]))
            return submit(null, {method: "post"})
        }

        isCastingBallot.current = true

        try {
            const {data} = await refetchElectionEvent()

            // Check if election events are available
            if (!(data && data.sequent_backend_election_event.length > 0)) {
                isCastingBallot.current = false
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.LOAD_ELECTION_EVENT}`))
                return submit({error: errorType}, {method: "post"})
            }

            const record = data?.sequent_backend_election_event?.[0]
            const eventStatus = record?.status as IElectionEventStatus | undefined

            // Check if election event is open for voting
            if (eventStatus?.voting_status !== EVotingStatus.OPEN) {
                isCastingBallot.current = false
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.ELECTION_EVENT_NOT_OPEN}`))
                return submit({error: errorType.toString()}, {method: "post"})
            }

            // Cast the vote using the stored ballot data
            let result = await insertCastVote({
                variables: {
                    electionId: ballotData.electionId,
                    ballotId: ballotData.ballotId,
                    content: ballotData.ballot,
                },
            })

            if (result.errors) {
                isCastingBallot.current = false
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.CAST_VOTE}`))
                return submit({error: errorType}, {method: "post"})
            }

            let newCastVote = result.data?.insert_cast_vote
            if (newCastVote) {
                dispatch(addCastVotes([newCastVote]))
            }

            // Clear the stored ballot data after successful casting
            removeBallotDataFromSessionStorage()
            return submit(null, {method: "post"})
        } catch (error) {
            isCastingBallot.current = false
            let castError = error as IGraphQLActionError
            if (castError?.graphQLErrors?.[0]?.extensions?.code) {
                let errorCode = castError?.graphQLErrors?.[0]?.extensions?.code
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.CAST_VOTE}_${errorCode}`))
            } else {
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.CAST_VOTE}`))
            }
            return submit({error: errorType}, {method: "post"})
        }
    }

    // Selectively clear ballot-related session data when component mounts
    useEffect(() => {
        // Only clear session data if we're not in the middle of a golden policy flow
        if (
            !(
                (!ballotStyle || !auditableBallot || !selectionState) &&
                isGoldenPolicy &&
                isGoldUser()
            )
        ) {
            removeBallotDataFromSessionStorage()
        }
    }, [])

    useEffect(() => {
        // Handle the golden user flow after reauthentication
        if ((!ballotStyle || !auditableBallot || !selectionState) && isGoldenPolicy) {
            if (isGoldUser()) {
                if (!isCastingBallot.current) {
                    isCastingBallot.current = true
                    try {
                        automaticCastBallot()
                    } catch (error) {
                        removeBallotDataFromSessionStorage()
                        console.error("Error casting ballot:", error)
                        setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.REAUTH_FAILED}`))
                    }
                }
            } else {
                // If not a gold user but golden policy is required, redirect to election chooser
                navigate(`/tenant/${tenantId}/event/${eventId}/election-chooser`)
            }
        }
    }, [ballotStyle, selectionState, auditableBallot, isGoldenPolicy])

    if (!ballotStyle || !auditableBallot) {
        return errorMsg ? (
            <Box sx={{margin: "auto 0"}}>
                <WarnBox variant="error">{errorMsg}</WarnBox>
                <Box
                    sx={{
                        display: "flex",
                        justifyContent: "center",
                        width: "100%",
                        marginTop: "16px",
                    }}
                >
                    <StyledLink to={backLink} sx={{width: {xs: "100%", sm: "200px"}}}>
                        <StyledButton sx={{width: {xs: "100%", sm: "200px"}}}>
                            <Icon icon={faAngleLeft} size="sm" />
                            <Box>{t("reviewScreen.backButton")}</Box>
                        </StyledButton>
                    </StyledLink>
                </Box>
            </Box>
        ) : (
            <CircularProgress />
        )
    }

    const contestsOrderType = ballotStyle?.ballot_eml.election_presentation?.contests_order
    const contests = sortContestList(ballotStyle.ballot_eml.contests, contestsOrderType)

    return (
        <PageLimit maxWidth="lg" className="review-screen screen">
            {auditButtonCfg === EVotingPortalAuditButtonCfg.NOT_SHOW ? null : (
                <BallotHash hash={ballotId || ""} onHelpClick={() => setOpenBallotIdHelp(true)} />
            )}
            <Dialog
                handleClose={handleCloseDialogIdHelp}
                open={openBallotIdHelp}
                title={t("reviewScreen.ballotIdHelpDialog.title")}
                ok={t("reviewScreen.ballotIdHelpDialog.ok")}
                maxWidth="md"
                middleActions={
                    auditButtonCfg === EVotingPortalAuditButtonCfg.SHOW_IN_HELP
                        ? [
                              <AuditButton
                                  key={"audit-button"}
                                  onClick={() => setAuditBallotHelp(true)}
                              />,
                          ]
                        : []
                }
                cancel={t("reviewScreen.ballotIdHelpDialog.cancel")}
                variant="info"
            >
                {stringToHtml(t("reviewScreen.ballotIdHelpDialog.content"))}
            </Dialog>
            {auditButtonCfg === EVotingPortalAuditButtonCfg.SHOW_IN_HELP ? (
                <AuditBallotHelpDialog
                    auditBallotHelp={auditBallotHelp}
                    handleClose={handleCloseDialogAuditHelp}
                />
            ) : null}
            <Box marginTop="48px">
                <Stepper selected={2} />
            </Box>
            <StyledTitle variant="h4" fontSize="24px" fontWeight="bold" sx={{margin: 0}}>
                <Box>{t("reviewScreen.title")}</Box>
                <IconButton
                    icon={faCircleQuestion}
                    sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                    fontSize="16px"
                    onClick={() => setReviewScreenHelp(true)}
                />
                <Dialog
                    handleClose={() => setReviewScreenHelp(false)}
                    open={openReviewScreenHelp}
                    title={t("reviewScreen.reviewScreenHelpDialog.title")}
                    ok={t("reviewScreen.reviewScreenHelpDialog.ok")}
                    variant="info"
                >
                    {stringToHtml(t("reviewScreen.reviewScreenHelpDialog.content"))}
                </Dialog>
            </StyledTitle>
            {errorMsg && <WarnBox variant="error">{errorMsg}</WarnBox>}
            <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                {stringToHtml(
                    auditButtonCfg === EVotingPortalAuditButtonCfg.NOT_SHOW ||
                        auditButtonCfg === EVotingPortalAuditButtonCfg.SHOW_IN_HELP
                        ? t("reviewScreen.descriptionNoAudit")
                        : t("reviewScreen.description")
                )}
            </Typography>
            {contests.map((question, index) => (
                <Box key={question.id} className={`contest-${index}`}>
                    <Question
                        ballotStyle={ballotStyle}
                        question={question}
                        isReview={true}
                        setDecodedContests={() => undefined}
                        errorSelectionState={errorSelectionState}
                    />
                </Box>
            ))}
            {!isCastingBallot.current && (
                <ActionButtons
                    ballotStyle={ballotStyle}
                    ballotId={ballotId ?? ""}
                    auditableBallot={auditableBallot}
                    auditButtonCfg={auditButtonCfg}
                    castVoteConfirmModal={castVoteConfirmModal}
                    setErrorMsg={setErrorMsg}
                />
            )}
        </PageLimit>
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
