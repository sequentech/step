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
    GRAPHQLRESPONSE_TIMEOUT_ERROR,
    IAuditableSingleBallot,
    IAuditableMultiBallot,
    ECastVoteGoldLevelPolicy,
    EElectionEventContestEncryptionPolicy,
    IHashableBallot,
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
import {useMutation, useQuery, ApolloError} from "@apollo/client"
import {INSERT_CAST_VOTE} from "../queries/InsertCastVote"
import {GetElectionEventQuery, InsertCastVoteMutation, GetElectionsQuery} from "../gql/graphql"
import {GET_ELECTIONS} from "../queries/GetElections"
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
import {GET_ELECTION_EVENT} from "../queries/GetElectionEvent"
import Stepper from "../components/Stepper"
import {selectBallotSelectionByElectionId} from "../store/ballotSelections/ballotSelectionsSlice"
import {
    sortContestList,
    hashBallot,
    hashMultiBallot,
    IHashableSingleBallot,
    IHashableMultiBallot,
} from "@sequentech/ui-core"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {AuthContext} from "../providers/AuthContextProvider"
import {useGetOne} from "react-admin"
import {TFunction} from "i18next"
import {
    SessionBallotData,
    clearSessionStorageBallotData,
    BALLOT_DATA_KEY,
    BALLOT_DATA_EXPIRATION_KEY,
} from "../store/castVotes/sessionBallotData"
import {setConfirmationScreenData} from "../store/castVotes/confirmationScreenDataSlice"
import { TIMEOUT } from "dns"

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

const useIsEventStatusOpen = (
    tenantId: string | undefined,
    eventId: string | undefined,
    setErrorMsg: (msg: CastBallotsErrorType) => void,
    t: TFunction,
    skip?: boolean
) => {
    const {refetch} = useQuery<GetElectionEventQuery>(GET_ELECTION_EVENT, {
        variables: {
            electionEventId: eventId,
            tenantId,
        },
        skip,
        onError: (error) => {
            if (error.networkError) {
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.NETWORK_ERROR}`))
            } else {
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.UNABLE_TO_FETCH_DATA}`))
            }
        },
    })

    return async () => {
        let data: GetElectionEventQuery | undefined
        try {
            const result = await refetch()
            data = (result as {data: GetElectionEventQuery}).data
        } catch (error) {
            console.log("Error fetching election event", error)
            if (error instanceof Error /* && error.message === "timeout" */) {
                // TODO: How to identify a timeout error?
                console.log("Error fetching election event", error as Error)
                console.log(error.name, error.message, error.cause)
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.REFETCH_TIMEOUT_ERROR}`))
            } else if (error instanceof ApolloError /*  && error.networkError */) {
                console.log("Error fetching election event", error as ApolloError)
                console.log(error.name, error.message, error.cause, error.networkError)
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.NETWORK_ERROR}`))
            } else {
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.UNABLE_TO_FETCH_DATA}`))
            }
            return false
        }

        if (!(data?.sequent_backend_election_event.length > 0)) {
            setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.LOAD_ELECTION_EVENT}`))
            return false
        }

        const record = data?.sequent_backend_election_event?.[0]
        const eventStatus = record?.status as IElectionEventStatus | undefined

        if (eventStatus?.voting_status !== EVotingStatus.OPEN) {
            setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.ELECTION_EVENT_NOT_OPEN}`))
            return false
        }
        return true
    }
}

const useTryInsertCastVote = () => {
    const {t} = useTranslation()
    const [insertCastVote] = useMutation<InsertCastVoteMutation>(INSERT_CAST_VOTE)
    const dispatch = useAppDispatch()

    return async (
        electionId: string,
        ballotId: string,
        content: string,
        setErrorMsg: (msg: CastBallotsErrorType) => void
    ) => {

        try {
            let result = await insertCastVote({
                variables: {
                    electionId,
                    ballotId,
                    content,
                },
            });

            if (result.errors) {
                console.log(result.errors.map((e) => e.message))
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.CAST_VOTE}`))
                return false
            }

            let newCastVote = result.data?.insert_cast_vote
            if (newCastVote) {
                dispatch(addCastVotes([newCastVote]))
            }
            return true
        } catch (error) {
            console.log(error)
            let castError = error as IGraphQLActionError
            let errorExtensions = castError?.graphQLErrors?.[0]?.extensions
            if (errorExtensions?.code) {
                let errorCode = errorExtensions?.code
                console.log(castError.name, castError.message)
                let internalErrMessage = castError?.graphQLErrors?.[0]?.extensions?.internal?.error?.message
                console.log(errorCode, internalErrMessage)
                if ( errorCode === "unexpected" && internalErrMessage === GRAPHQLRESPONSE_TIMEOUT_ERROR ) {
                    setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.CAST_VOTE_TIMEOUT}`))
                } else {
                    setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.CAST_VOTE}_${errorCode}`))
                }

            } else if (error instanceof ApolloError  && error.networkError ) {
                console.log(error.name, error.message, error.cause, error.networkError)
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.NETWORK_ERROR}`))

            } else if (castError?.message?.includes("internal error")) {
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.INTERNAL_ERROR}`)) // can happen if the backend panics
            } else {
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.CAST_VOTE}`)) // Generic error
            }
            return false
        }
    }
}

const ActionButtons: React.FC<ActionButtonProps> = ({
    ballotStyle,
    auditableBallot,
    auditButtonCfg,
    castVoteConfirmModal,
    ballotId,
    setErrorMsg,
}) => {
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
    const addFakeCastVote = useAddFakeCastVote(tenantId, eventId)
    const isEventStatusOpen = useIsEventStatusOpen(
        tenantId,
        eventId,
        setErrorMsg,
        t,
        globalSettings.DISABLE_AUTH
    )
    const tryInsertCastVote = useTryInsertCastVote()

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

    const storeBallotDataAndReauth = async (ballotData: SessionBallotData) => {
        const errorType = VotingPortalErrorType.UNABLE_TO_CAST_BALLOT
        try {
            // Set a 5-minute expiration for security
            const FIVE_MINUTES = 5 * 60 * 1000
            // Store the data with expiration info
            sessionStorage.setItem(BALLOT_DATA_KEY, JSON.stringify(ballotData))
            sessionStorage.setItem(
                BALLOT_DATA_EXPIRATION_KEY,
                (Date.now() + FIVE_MINUTES).toString()
            )
        } catch (e) {
            console.error("Error saving ballotData to sessionStorage", e)
            clearSessionStorageBallotData()
            setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.SESSION_STORAGE_ERROR}`))
            return submit({error: errorType}, {method: "post"})
        }
        try {
            const baseUrl = new URL(window.location.href)
            await reauthWithGold(baseUrl.toString())
            return submit(null, {method: "post"})
        } catch (error) {
            console.error("Re-authentication failed:", error)
            clearSessionStorageBallotData()
            setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.REAUTH_FAILED}`))
            return submit({error: errorType}, {method: "post"})
        }
    }

    const castBallotAction = async () => {
        const errorType = VotingPortalErrorType.UNABLE_TO_CAST_BALLOT
        isCastingBallot.current = true
        if (!(await isEventStatusOpen())) {
            isCastingBallot.current = false
            return submit({error: errorType.toString()}, {method: "post"})
        }

        const isGoldenPolicy =
            ballotStyle?.ballot_eml.election_presentation?.cast_vote_gold_level ===
            ECastVoteGoldLevelPolicy.GOLD_LEVEL
        if (isDemo || globalSettings.DISABLE_AUTH) {
            if (isGoldenPolicy) {
                // Save contests to session storage and perform reauthentication
                const ballotData: SessionBallotData = {
                    ballotId,
                    electionId: ballotStyle.election_id,
                    isDemo: true,
                    ballot: JSON.stringify("{}"),
                    timestamp: Date.now(), // Add timestamp for expiration check
                }
                return await storeBallotDataAndReauth(ballotData)
            } else {
                addFakeCastVote()
                return submit(null, {method: "post"})
            }
        }

        const isMultiContest =
            auditableBallot?.config.election_event_presentation?.contest_encryption_policy ==
            EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS

        let hashableBallot: IHashableSingleBallot | IHashableMultiBallot | undefined
        try {
            hashableBallot = isMultiContest
                ? toHashableMultiBallot(auditableBallot as IAuditableMultiBallot)
                : toHashableBallot(auditableBallot as IAuditableSingleBallot)
        } catch (e) {
            isCastingBallot.current = false
            setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.TO_HASHABLE_BALLOT_ERROR}`))
            return submit({error: errorType}, {method: "post"})
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
                electionId: ballotStyle.election_id,
                isDemo,
                ballot: JSON.stringify(hashableBallot),
                timestamp: Date.now(), // Add timestamp for expiration check
            }
            return await storeBallotDataAndReauth(ballotData)
        }

        if (
            !(await tryInsertCastVote(
                ballotStyle.election_id,
                ballotId,
                JSON.stringify(hashableBallot),
                setErrorMsg
            ))
        ) {
            isCastingBallot.current = false
            return submit({error: errorType}, {method: "post"})
        }
        return submit(null, {method: "post"})
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
    const {electionId} = useParams<{electionId?: string}>()
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionId)))
    const location = useLocation()
    const auditableBallot = useAppSelector(selectAuditableBallot(String(electionId)))
    const [auditBallotHelp, setAuditBallotHelp] = useState<boolean>(false)
    const [openBallotIdHelp, setOpenBallotIdHelp] = useState(false)
    const [openReviewScreenHelp, setReviewScreenHelp] = useState(false)
    const {interpretContestSelection, interpretMultiContestSelection} = provideBallotService()
    const {t} = useTranslation()
    const backLink = useRootBackLink()
    const navigate = useNavigate()
    const submit = useSubmit()
    const {tenantId, eventId} = useParams<TenantEventType>()
    const [errorMsg, setErrorMsg] = useState<CastBallotsErrorType>()
    const authContext = useContext(AuthContext)
    const {isGoldUser, reauthWithGold} = authContext
    const isCastingBallot = useRef<boolean>(false)
    const {globalSettings} = useContext(SettingsContext)
    const [insertCastVote] = useMutation<InsertCastVoteMutation>(INSERT_CAST_VOTE)
    const dispatch = useAppDispatch()
    const addFakeCastVote = useAddFakeCastVote(tenantId, eventId)
    const isEventStatusOpen = useIsEventStatusOpen(
        tenantId,
        eventId,
        setErrorMsg,
        t,
        globalSettings.DISABLE_AUTH
    )
    const tryInsertCastVote = useTryInsertCastVote()

    const {data: dataElections} = useQuery<GetElectionsQuery>(GET_ELECTIONS, {
        variables: {
            electionIds: electionId ? [electionId] : [],
        },
        skip: globalSettings.DISABLE_AUTH, // Skip query if in demo mode
    })

    const isGoldenPolicy = dataElections?.sequent_backend_election.some(
        (item) => item.presentation?.cast_vote_gold_level === ECastVoteGoldLevelPolicy.GOLD_LEVEL
    )

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

    const getBallotDataFromSessionStorage = () => {
        let ballotData: SessionBallotData | undefined
        let expirationTime: number | undefined
        const storedExpirationTime = sessionStorage.getItem(BALLOT_DATA_EXPIRATION_KEY)
        const storedData = sessionStorage.getItem(BALLOT_DATA_KEY)

        if (!storedExpirationTime || !storedData) {
            setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.SESSION_STORAGE_ERROR}`))
            return undefined
        }

        expirationTime = parseInt(storedExpirationTime, 10)
        if (expirationTime && Date.now() > expirationTime) {
            // Data has expired, clean up and return error
            console.error("Re-authentication failed: EXPIRED")
            setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.SESSION_EXPIRED}`))
            return undefined
        }

        try {
            ballotData = JSON.parse(storedData) as SessionBallotData
        } catch (error) {
            setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.PARSE_BALLOT_DATA_ERROR}`))
            return undefined
        }

        if (!ballotData || !ballotData.ballotId || !ballotData.electionId || !ballotData.ballot) {
            setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.NOT_VALID_BALLOT_DATA_ERROR}`))
            return undefined
        }

        return ballotData
    }

    const automaticCastBallot = async () => {
        isCastingBallot.current = true
        const errorType = VotingPortalErrorType.UNABLE_TO_CAST_BALLOT
        const ballotData = getBallotDataFromSessionStorage()

        if (!ballotData) {
            isCastingBallot.current = false
            clearSessionStorageBallotData()
            return submit({error: errorType}, {method: "post"})
        }

        if (ballotData?.isDemo) {
            addFakeCastVote()
            clearSessionStorageBallotData()
            isCastingBallot.current = false
            return submit(null, {method: "post"})
        }

        if (!(await isEventStatusOpen())) {
            isCastingBallot.current = false
            return submit({error: errorType.toString()}, {method: "post"})
        }

        if (
            !(await tryInsertCastVote(
                ballotData.electionId,
                ballotData.ballotId,
                ballotData.ballot,
                setErrorMsg
            ))
        ) {
            isCastingBallot.current = false
            return submit({error: errorType}, {method: "post"})
        }

        // set ConfirmationScreenData (ballotId and isDemo) to a new object in redux state, so it can be read later on from the confirmation screen
        dispatch(
            setConfirmationScreenData({
                electionId: ballotData.electionId,
                confirmationScreenData: {
                    ballotId: ballotData.ballotId,
                    isDemo: ballotData.isDemo,
                },
            })
        )
        clearSessionStorageBallotData()
        return submit(null, {method: "post"})
    }

    // Selectively clear ballot-related session data when component mounts
    useEffect(() => {
        // Only clear session data if we're not in the middle of a golden policy flow
        if (!isGoldUser() && !ballotStyle && !auditableBallot && !selectionState) {
            clearSessionStorageBallotData()
        }
    }, [])

    useEffect(() => {
        if ((!ballotStyle || !auditableBallot || !selectionState) && isGoldenPolicy) {
            if (isGoldUser()) {
                if (!isCastingBallot.current) {
                    automaticCastBallot()
                    clearSessionStorageBallotData()
                }
            } else {
                // If not a gold user but golden policy is required, redirect to election chooser
                navigate(`/tenant/${tenantId}/event/${eventId}/election-chooser`)
            }
        } else {
            console.log("Normal flow")
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
                    auditableBallot={auditableBallot}
                    auditButtonCfg={auditButtonCfg}
                    castVoteConfirmModal={castVoteConfirmModal}
                    ballotId={ballotId ?? ""}
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
