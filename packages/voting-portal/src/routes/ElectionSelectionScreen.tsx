// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, Button, CircularProgress, Typography, Alert} from "@mui/material"
import React, {useContext, useEffect, useMemo, useState} from "react"
import {useTranslation} from "react-i18next"
import {Dialog, IconButton, PageLimit, SelectElection, theme} from "@sequentech/ui-essentials"
import {
    isString,
    stringToHtml,
    translateElection,
    EVotingStatus,
    IElectionEventStatus,
    isUndefined,
    IElectionStatus,
    EEarlyVotingPolicy,
    IAreaPresentation,
} from "@sequentech/ui-core"
import {AuthContext} from "../providers/AuthContextProvider"
import {faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import {styled} from "@mui/material/styles"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {
    IBallotStyle,
    selectBallotStyleByElectionId,
    selectBallotStyleElectionIds,
    selectFirstBallotStyle,
    setBallotStyle,
} from "../store/ballotStyles/ballotStylesSlice"
import {resetBallotSelection} from "../store/ballotSelections/ballotSelectionsSlice"
import {selectElectionById, setElection, selectElectionIds} from "../store/elections/electionsSlice"
import {AppDispatch} from "../store/store"
import {addCastVotes, selectCastVotesByElectionId} from "../store/castVotes/castVotesSlice"
import {useLocation, useNavigate, useParams} from "react-router-dom"
import {useQuery} from "@apollo/client/react"
import {GET_BALLOT_STYLES} from "../queries/GetBallotStyles"
import {
    GetBallotStylesQuery,
    GetCastVotesQuery,
    GetElectionEventQuery,
    GetElectionsQuery,
    GetSupportMaterialsQuery,
} from "../gql/graphql"
import {GET_ELECTIONS} from "../queries/GetElections"
import {ELECTIONS_LIST} from "../fixtures/election"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {GET_ELECTION_EVENT} from "../queries/GetElectionEvent"
import {GET_CAST_VOTES} from "../queries/GetCastVotes"
import {
    ElectionScreenErrorType,
    ElectionScreenMsgType,
    VotingPortalError,
    VotingPortalErrorType,
} from "../services/VotingPortalError"
import {
    IElectionEvent,
    selectElectionEventById,
    setElectionEvent,
} from "../store/electionEvents/electionEventsSlice"
import {TenantEventType} from ".."
import Stepper from "../components/Stepper"
import {clearIsVoted, selectBypassChooser, setBypassChooser} from "../store/extra/extraSlice"
import {updateBallotStyleAndSelection} from "../services/BallotStyles"
import useUpdateTranslation from "../hooks/useUpdateTranslation"
import {GET_SUPPORT_MATERIALS} from "../queries/GetSupportMaterials"
import {setSupportMaterial} from "../store/supportMaterials/supportMaterialsSlice"

const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
    font-size: 24px;
    font-weight: 500;
    line-height: 27px;
    margin-top: 20px;
    margin-bottom: 16px;
`

const ElectionContainer = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 30px;
    margin-bottom: 30px;
`

interface ElectionWrapperProps {
    electionId: string
    bypassChooser: boolean
    canVoteTest: boolean
}

const isElectionEventOnlineVotingOpen = (electionEvent?: IElectionEvent): boolean => {
    return (
        ((electionEvent?.status as IElectionEventStatus | null)?.voting_status ??
            EVotingStatus.CLOSED) === EVotingStatus.OPEN
    )
}

const isElectionEventKioskOpen = (electionEvent?: IElectionEvent): boolean => {
    return (
        ((electionEvent?.status as IElectionEventStatus | null)?.kiosk_voting_status ??
            EVotingStatus.CLOSED) === EVotingStatus.OPEN
    )
}

const isElectionEventEarlyVotingOpen = (electionEvent?: IElectionEvent): boolean => {
    return (
        ((electionEvent?.status as IElectionEventStatus | null)?.early_voting_status ??
            EVotingStatus.CLOSED) === EVotingStatus.OPEN
    )
}

const ElectionWrapper: React.FC<ElectionWrapperProps> = ({
    electionId,
    bypassChooser,
    canVoteTest,
}) => {
    const navigate = useNavigate()
    const location = useLocation()
    const {i18n} = useTranslation()

    const {tenantId, eventId} = useParams<TenantEventType>()
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const election = useAppSelector(selectElectionById(electionId))
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(electionId))
    const castVotes = useAppSelector(selectCastVotesByElectionId(String(electionId)))
    const [visitedBypassChooser, setVisitedBypassChooser] = useState(false)
    const authContext = useContext(AuthContext)
    const isKiosk = authContext.isKiosk()

    if (!election) {
        throw new VotingPortalError(VotingPortalErrorType.INTERNAL_ERROR)
    }

    const electionStatus = election?.status as IElectionStatus | null
    const isVotingOpen = () => {
        let isOnlineVotingOpen: boolean =
            (electionStatus?.voting_status as EVotingStatus) === EVotingStatus.OPEN

        if (isKiosk) {
            return isKioskOpen() && isElectionEventKioskOpen(electionEvent)
        } else {
            return (
                (isOnlineVotingOpen && isElectionEventOnlineVotingOpen(electionEvent)) ||
                (isEarlyVotingOpen() && isElectionEventEarlyVotingOpen(electionEvent))
            )
        }
    }

    const isKioskOpen = () => {
        return (electionStatus?.kiosk_voting_status as EVotingStatus) === EVotingStatus.OPEN
    }

    const isEarlyVotingPolicyEnabled = () => {
        let area_presentation = ballotStyle?.ballot_eml?.area_presentation as IAreaPresentation
        return area_presentation.allow_early_voting === EEarlyVotingPolicy.ALLOW_EARLY_VOTING
    }
    const isEarlyVotingOpen = () => {
        let isOpen = electionStatus?.early_voting_status === EVotingStatus.OPEN
        return isEarlyVotingPolicyEnabled() && isOpen
    }

    const isVotingStarted = () => {
        if (isKiosk) {
            return electionStatus?.kiosk_voting_status !== EVotingStatus.NOT_STARTED
        } else {
            return (
                electionStatus?.voting_status !== EVotingStatus.NOT_STARTED ||
                (isEarlyVotingPolicyEnabled() &&
                    electionStatus?.early_voting_status !== EVotingStatus.NOT_STARTED)
            )
        }
    }

    const isPreview = sessionStorage.getItem("isDemo") === "true"
    const canVote = () => {
        if (!canVoteTest && !election.name?.includes("TEST")) {
            return false
        }

        if (ballotStyle?.ballot_eml.num_allowed_revotes === 0) {
            return true
        }

        return (
            isPreview ||
            (castVotes.length < (ballotStyle?.ballot_eml.num_allowed_revotes ?? 1) &&
                isVotingOpen())
        )
    }

    const onClickToVote = () => {
        if (!canVote() || (!isPreview && !isVotingOpen())) {
            console.log("cannot vote")
            return
        }
        navigate(
            `/tenant/${tenantId}/event/${eventId}/election/${electionId}/start${location.search}`
        )
    }

    const handleClickBallotLocator = () => {
        navigate(`../election/${electionId}/ballot-locator${location.search}`)
    }

    useEffect(() => {
        if (visitedBypassChooser) {
            console.log("visitedBypassChooser")
            return
        }
        if (bypassChooser && ballotStyle) {
            console.log("setVisitedBypassChooser")
            setVisitedBypassChooser(true)
            onClickToVote()
        }
    }, [bypassChooser, visitedBypassChooser, setVisitedBypassChooser, ballotStyle])

    return (
        <SelectElection
            isActive={canVote()}
            isOpen={isVotingOpen()}
            title={translateElection(election, "name", i18n.language) || "-"}
            hasVoted={castVotes.length > 0}
            onClickToVote={canVote() ? onClickToVote : undefined}
            onClickBallotLocator={handleClickBallotLocator}
            electionDates={ballotStyle?.ballot_eml?.election_dates}
            isStarted={isVotingStarted()}
        />
    )
}

const fakeUpdateBallotStyleAndSelection = (dispatch: AppDispatch) => {
    for (let election of ELECTIONS_LIST) {
        try {
            const formattedBallotStyle: IBallotStyle = {
                id: election.id,
                election_id: election.id,
                election_event_id: election.id,
                tenant_id: election.id,
                ballot_eml: election,
                ballot_signature: null,
                created_at: "",
                area_id: election.id,
                annotations: null,
                labels: null,
                last_updated_at: "",
            }
            dispatch(setElection({...election, image_document_id: ""}))
            dispatch(setBallotStyle(formattedBallotStyle))
            dispatch(clearIsVoted())
            dispatch(
                resetBallotSelection({
                    ballotStyle: formattedBallotStyle,
                })
            )
        } catch (error) {
            console.log(`Error loading fake EML: ${error}`, election)
            throw new VotingPortalError(VotingPortalErrorType.INTERNAL_ERROR)
        }
    }
}

const ElectionSelectionScreen: React.FC = () => {
    const {t} = useTranslation()
    const navigate = useNavigate()
    const location = useLocation()

    const {globalSettings, defaultLanguageTouched, setDefaultLanguageTouched} =
        useContext(SettingsContext)
    const {eventId, tenantId} = useParams<{eventId?: string; tenantId?: string}>()
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const oneBallotStyle = useAppSelector(selectFirstBallotStyle)
    useUpdateTranslation({electionEvent}, defaultLanguageTouched, setDefaultLanguageTouched) // Overwrite translations
    const ballotStyleElectionIds = useAppSelector(selectBallotStyleElectionIds)
    const electionIds = useAppSelector(selectElectionIds)
    const dispatch = useAppDispatch()
    const [canVoteTest, setCanVoteTest] = useState<boolean>(true)
    const [testElectionId, setTestElectionId] = useState<string | null>(null)
    const castVotesTestElection = useAppSelector(
        selectCastVotesByElectionId(String(testElectionId || tenantId))
    )
    const [openChooserHelp, setOpenChooserHelp] = useState(false)
    const [isMaterialsActivated, setIsMaterialsActivated] = useState<boolean>(false)
    const bypassChooser = useAppSelector(selectBypassChooser())
    const [errorMsg, setErrorMsg] = useState<VotingPortalErrorType | ElectionScreenErrorType>()
    const [alertMsg, setAlertMsg] = useState<ElectionScreenMsgType>()

    const {
        error: errorBallotStyles,
        data: dataBallotStyles,
        loading: loadingBallotStyles,
    } = useQuery<GetBallotStylesQuery>(GET_BALLOT_STYLES, {
        skip: globalSettings.DISABLE_AUTH, // Skip query if in demo mode
    })

    const {
        error: errorElections,
        data: dataElections,
        loading: loadingElections,
    } = useQuery<GetElectionsQuery>(GET_ELECTIONS, {
        variables: {
            electionIds: ballotStyleElectionIds,
        },
        skip: globalSettings.DISABLE_AUTH, // Skip query if in demo mode
    })

    const {
        error: errorElectionEvent,
        data: dataElectionEvent,
        loading: loadingElectionEvent,
    } = useQuery<GetElectionEventQuery>(GET_ELECTION_EVENT, {
        variables: {
            electionEventId: eventId,
            tenantId,
        },
        skip: globalSettings.DISABLE_AUTH, // Skip query if in demo mode
    })

    // Materials
    const {
        data: dataMaterials,
        error: errorMaterials,
        loading: loadingMaterials,
    } = useQuery<GetSupportMaterialsQuery>(GET_SUPPORT_MATERIALS, {
        variables: {
            electionEventId: eventId || "",
            tenantId: tenantId || "",
        },
        skip: globalSettings.DISABLE_AUTH || !isMaterialsActivated, // Skip query if in demo mode
    })

    const {data: castVotes, error: errorCastVote} = useQuery<GetCastVotesQuery>(GET_CAST_VOTES, {
        skip: globalSettings.DISABLE_AUTH,
    })

    const handleNavigateMaterials = () => {
        navigate(`/tenant/${tenantId}/event/${eventId}/materials${location.search}`)
    }

    const hasNoElections = !loadingElections && dataElections?.sequent_backend_election.length === 0
    const isPublished = useMemo(
        () => !!dataElectionEvent?.sequent_backend_election_event[0].status?.is_published,
        [dataElectionEvent?.sequent_backend_election_event]
    )

    useEffect(() => {
        if (!dataMaterials || globalSettings.DISABLE_AUTH || !isMaterialsActivated) {
            return
        }

        for (let material of dataMaterials.sequent_backend_support_material) {
            dispatch(setSupportMaterial(material))
        }
    }, [dataMaterials, globalSettings.DISABLE_AUTH, isMaterialsActivated])

    // Errors handling
    useEffect(() => {
        if (globalSettings.DISABLE_AUTH) {
            return
        }
        if (errorElections || errorElectionEvent || errorBallotStyles || errorCastVote) {
            if (errorBallotStyles?.message.includes("x-hasura-area-id")) {
                setErrorMsg(t(`electionSelectionScreen.errors.${ElectionScreenErrorType.NO_AREA}`))
            } else if (
                errorElections?.networkError ||
                errorElectionEvent?.networkError ||
                errorBallotStyles?.networkError ||
                errorCastVote?.networkError
            ) {
                setErrorMsg(t(`electionSelectionScreen.errors.${ElectionScreenErrorType.NETWORK}`))
            } else {
                setErrorMsg(
                    t(`electionSelectionScreen.errors.${ElectionScreenErrorType.FETCH_DATA}`)
                )
            }
        } else if (dataElectionEvent?.sequent_backend_election_event.length === 0) {
            setErrorMsg(
                t(`electionSelectionScreen.errors.${ElectionScreenErrorType.NO_ELECTION_EVENT}`)
            )
        } else if (!isPublished) {
            setAlertMsg(t(`electionSelectionScreen.alerts.${ElectionScreenMsgType.NOT_PUBLISHED}`))
        } else if (hasNoElections) {
            if (electionIds.length > 0) {
                setErrorMsg(
                    t(
                        `electionSelectionScreen.errors.${ElectionScreenErrorType.OBTAINING_ELECTION}`,
                        {electionIds: JSON.stringify(electionIds)}
                    )
                )
            } else {
                setAlertMsg(
                    t(`electionSelectionScreen.alerts.${ElectionScreenMsgType.NO_ELECTIONS}`)
                )
            }
        } else {
            setAlertMsg(undefined)
            setErrorMsg(undefined)
        }
    }, [
        errorBallotStyles,
        errorCastVote,
        errorElectionEvent,
        errorElections,
        isPublished,
        hasNoElections,
        dataElectionEvent,
        globalSettings.DISABLE_AUTH,
    ])

    useEffect(() => {
        if (dataBallotStyles && dataBallotStyles.sequent_backend_ballot_style.length > 0) {
            try {
                updateBallotStyleAndSelection(dataBallotStyles, dispatch)
            } catch {
                setErrorMsg(
                    t(`electionSelectionScreen.errors.${ElectionScreenErrorType.BALLOT_STYLES_EML}`)
                )
            }
        } else if (globalSettings.DISABLE_AUTH) {
            //fakeUpdateBallotStyleAndSelection(dispatch)
        }
    }, [globalSettings.DISABLE_AUTH, dataBallotStyles, dispatch])

    useEffect(() => {
        if (dataElections && dataElections.sequent_backend_election.length > 0) {
            for (let election of dataElections.sequent_backend_election) {
                dispatch(
                    setElection({
                        ...election,
                        image_document_id: "",
                        contests: [],
                        description: election.description ?? undefined,
                        alias: election.alias ?? undefined,
                    })
                )
            }

            let foundTestElection = dataElections.sequent_backend_election.find((election) =>
                election.name.includes("TEST")
            )

            if (foundTestElection) {
                setCanVoteTest(false)
            }

            setTestElectionId(foundTestElection?.id || null)
        }
    }, [dataElections, dispatch])

    useEffect(() => {
        if (!testElectionId) {
            return
        }
        setCanVoteTest(castVotesTestElection.length > 0)
    }, [castVotesTestElection, testElectionId, setCanVoteTest])

    useEffect(() => {
        const record = dataElectionEvent?.sequent_backend_election_event?.[0]
        if (record) {
            dispatch(setElectionEvent(record))
        }
    }, [dataElectionEvent, dispatch])

    useEffect(() => {
        setIsMaterialsActivated(electionEvent?.presentation?.materials?.activated || false)
    }, [electionEvent?.presentation?.materials?.activated])

    useEffect(() => {
        if (castVotes?.sequent_backend_cast_vote) {
            dispatch(addCastVotes(castVotes.sequent_backend_cast_vote))
        }
    }, [castVotes, dispatch])

    useEffect(() => {
        const skipPolicy =
            oneBallotStyle?.ballot_eml.election_event_presentation?.skip_election_list ?? false
        console.log("skipPolicy", skipPolicy)
        const newBypassChooser =
            skipPolicy &&
            1 === electionIds.length &&
            !errorCastVote &&
            !isUndefined(castVotes) &&
            !!electionEvent &&
            !!dataElections

        if (newBypassChooser && !bypassChooser) {
            console.log("new baypass chooser", newBypassChooser)
            dispatch(setBypassChooser(newBypassChooser))
        }
    }, [
        castVotes,
        electionIds,
        errorCastVote,
        castVotes,
        electionEvent,
        dataElections,
        oneBallotStyle,
    ])

    if (loadingElectionEvent || loadingElections || loadingBallotStyles) return <CircularProgress />

    return (
        <PageLimit maxWidth="lg" className="election-selection-screen screen">
            <Box marginTop="48px">
                <Stepper selected={0} />
            </Box>

            <Box
                sx={{
                    display: "flex",
                    flexDirection: "row",
                    justifyContent: "space-between",
                    alignItems: "center",
                    minHeight: "100px",
                }}
                className="title-section"
            >
                <Box sx={{width: "100%"}}>
                    <StyledTitle variant="h1">
                        <Box>{t("electionSelectionScreen.title")}</Box>
                        <IconButton
                            icon={faCircleQuestion}
                            sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                            fontSize="16px"
                            onClick={() => setOpenChooserHelp(true)}
                        />
                        <Dialog
                            handleClose={() => setOpenChooserHelp(false)}
                            open={openChooserHelp}
                            title={t("electionSelectionScreen.chooserHelpDialog.title")}
                            ok={t("electionSelectionScreen.chooserHelpDialog.ok")}
                            variant="info"
                        >
                            {stringToHtml(t("electionSelectionScreen.chooserHelpDialog.content"))}
                        </Dialog>
                    </StyledTitle>
                    {errorMsg || alertMsg ? (
                        <Alert severity="warning">{errorMsg || alertMsg}</Alert>
                    ) : (
                        <Typography
                            variant="body1"
                            sx={{color: theme.palette.customGrey.contrastText}}
                        >
                            {stringToHtml(t("electionSelectionScreen.description"))}
                        </Typography>
                    )}
                </Box>
                {isMaterialsActivated ? (
                    <Button onClick={handleNavigateMaterials}>{t("materials.common.label")}</Button>
                ) : null}
            </Box>
            <ElectionContainer className="elections-list">
                {!hasNoElections ? (
                    electionIds.map((electionId) => (
                        <ElectionWrapper
                            electionId={electionId}
                            key={electionId}
                            bypassChooser={bypassChooser}
                            canVoteTest={canVoteTest}
                        />
                    ))
                ) : (
                    <Box sx={{margin: "auto"}}>
                        <Typography>{t("electionSelectionScreen.noResults")}</Typography>
                    </Box>
                )}
            </ElectionContainer>
        </PageLimit>
    )
}

export default ElectionSelectionScreen
