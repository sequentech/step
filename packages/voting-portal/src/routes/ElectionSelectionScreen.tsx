// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, Button, CircularProgress, Typography, Alert} from "@mui/material"
import React, {useContext, useEffect, useMemo, useState} from "react"
import {Trans, useTranslation} from "react-i18next"
import {Dialog, IconButton, PageLimit, SelectElection, theme} from "@sequentech/ui-essentials"
import {
    isString,
    stringToHtml,
    translateFromPresentation,
    EVotingStatus,
    IElectionEventStatus,
    isUndefined,
    IElectionStatus,
    EEarlyVotingPolicy,
    IAreaPresentation,
    EResultsWebsiteStatus,
    parseResultsWebsitePolicy,
    formatVotingPortalDateTime,
    ESupportMaterialsPolicy,
    getEffectiveSupportMaterialsPolicy,
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
import {
    addCastVotes,
    CastVoteStatus,
    selectCastVotesByElectionId,
} from "../store/castVotes/castVotesSlice"
import {Link as RouterLink, useLocation, useNavigate, useParams} from "react-router-dom"
import {useQuery} from "@apollo/client/react"
import {GET_BALLOT_STYLES} from "../queries/GetBallotStyles"
import {
    GetBallotStylesQuery,
    GetCastVotesQuery,
    GetElectionEventQuery,
    GetElectionsQuery,
    GetSupportMaterialsQuery,
    GetSupportMaterialsAcknowledgmentQuery,
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
import {BallotStyleConfigurationError} from "../services/BallotStyles"
import useUpdateTranslation from "../hooks/useUpdateTranslation"
import {GET_SUPPORT_MATERIALS} from "../queries/GetSupportMaterials"
import {GET_SUPPORT_MATERIALS_ACKNOWLEDGMENT} from "../queries/GetSupportMaterialsAcknowledgment"
import {setSupportMaterial} from "../store/supportMaterials/supportMaterialsSlice"
import {useElectionClassName} from "../hooks/useElectionClassName"

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

const MaterialsGateLink = styled(RouterLink)`
    color: inherit;
    font-weight: 500;
    text-decoration: underline;
`

const TitleSection = styled(Box)`
    display: flex;
    flex-direction: row;
    justify-content: space-between;
    align-items: center;
    gap: 32px;
    min-height: 100px;

    @media (max-width: ${({theme}) => theme.breakpoints.values.sm}px) {
        flex-direction: column;
        align-items: stretch;
        gap: 16px;
        min-height: unset;
        padding: 24px 0;
    }
`

const PageActions = styled(Box)`
    display: flex;
    align-items: center;
    flex-shrink: 0;
    gap: 16px;

    .election-event-results-button {
        min-width: 150px;
        padding: 10px 24px;
        justify-content: center;
        font-weight: 500;
        line-height: 24px;
        white-space: nowrap;
    }

    @media (max-width: ${({theme}) => theme.breakpoints.values.sm}px) {
        width: 100%;

        > .MuiButton-root {
            flex: 1;
        }
    }
`

interface ElectionWrapperProps {
    electionId: string
    bypassChooser: boolean
    canVoteTest: boolean
    materialsGate: boolean
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

const isResultsWebsiteEnabled = (electionEvent?: IElectionEvent): boolean => {
    return (
        parseResultsWebsitePolicy(electionEvent?.presentation?.results_website)?.status ===
        EResultsWebsiteStatus.ENABLED
    )
}

const isElectionEventVotingClosed = (electionEvent?: IElectionEvent): boolean => {
    return (
        !isElectionEventOnlineVotingOpen(electionEvent) &&
        !isElectionEventKioskOpen(electionEvent) &&
        !isElectionEventEarlyVotingOpen(electionEvent)
    )
}

const ElectionWrapper: React.FC<ElectionWrapperProps> = ({
    electionId,
    bypassChooser,
    canVoteTest,
    materialsGate,
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
    const {globalSettings} = useContext(SettingsContext)
    const isKiosk = authContext.isKiosk()
    let [getElectionClassName] = useElectionClassName()

    if (!election) {
        throw new VotingPortalError(VotingPortalErrorType.INTERNAL_ERROR)
    }

    const defaultLanguageCode =
        election.presentation?.language_conf?.default_language_code ??
        electionEvent?.presentation?.language_conf?.default_language_code
    let electionClassName = getElectionClassName(election)

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
        if (materialsGate) {
            return false
        }

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

    const resultsUrl =
        globalSettings.RESULTS_PORTAL_URL &&
        eventId &&
        !isVotingOpen() &&
        isResultsWebsiteEnabled(electionEvent)
            ? `${globalSettings.RESULTS_PORTAL_URL.replace(/\/+$/, "")}/${eventId}/elections/${electionId}`
            : undefined

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
            title={
                translateFromPresentation(election, "name", i18n.language, {
                    defaultLanguageCode,
                }) || "-"
            }
            hasVoted={castVotes.length > 0}
            onClickToVote={canVote() ? onClickToVote : undefined}
            onClickBallotLocator={handleClickBallotLocator}
            resultsUrl={resultsUrl}
            electionDates={ballotStyle?.ballot_eml?.election_dates}
            isStarted={isVotingStarted()}
            className={electionClassName}
            formatDateTime={(input) =>
                formatVotingPortalDateTime(
                    input,
                    electionEvent,
                    i18n.resolvedLanguage || i18n.language
                )
            }
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
    const {t, i18n} = useTranslation()
    const navigate = useNavigate()
    const location = useLocation()

    const {globalSettings, defaultLanguageTouched, setDefaultLanguageTouched} =
        useContext(SettingsContext)
    const {eventId, tenantId} = useParams<{eventId?: string; tenantId?: string}>()
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const eventDefaultLanguageCode =
        electionEvent?.presentation?.language_conf?.default_language_code
    const oneBallotStyle = useAppSelector(selectFirstBallotStyle)
    //Handle both transalations from presentation and i18n language change.
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
    const [materialsPolicy, setMaterialsPolicy] = useState<ESupportMaterialsPolicy>(
        ESupportMaterialsPolicy.OFF
    )
    const isMaterialsVisible = materialsPolicy !== ESupportMaterialsPolicy.OFF
    const isMaterialsMandatory = materialsPolicy === ESupportMaterialsPolicy.MANDATORY_FOR_VOTING
    const bypassChooser = useAppSelector(selectBypassChooser())
    const [errorMsg, setErrorMsg] = useState<ElectionScreenErrorType>()
    const [errorMsgElectionIds, setErrorMsgElectionIds] = useState<string | undefined>(undefined)
    const [ballotStyleConfigurationError, setBallotStyleConfigurationError] = useState<
        | {
              translationKey: string
              translationParams: Record<string, string>
          }
        | undefined
    >(undefined)
    const [alertMsg, setAlertMsg] = useState<ElectionScreenMsgType>()
    const eventResultsUrl =
        globalSettings.RESULTS_PORTAL_URL &&
        eventId &&
        isResultsWebsiteEnabled(electionEvent) &&
        isElectionEventVotingClosed(electionEvent)
            ? `${globalSettings.RESULTS_PORTAL_URL.replace(/\/+$/, "")}/${eventId}`
            : undefined

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
        skip: globalSettings.DISABLE_AUTH || !isMaterialsVisible, // Skip query if in demo mode
    })

    const {
        data: dataMaterialsAcknowledgment,
        error: errorMaterialsAcknowledgment,
        loading: loadingMaterialsAcknowledgment,
    } = useQuery<GetSupportMaterialsAcknowledgmentQuery>(GET_SUPPORT_MATERIALS_ACKNOWLEDGMENT, {
        variables: {
            electionEventId: eventId || "",
        },
        // Always re-check on mount: the voter may have just acknowledged on the
        // Support Materials screen and navigated back here, and a cached "not
        // acknowledged yet" result must not re-gate the Ballot list.
        fetchPolicy: "network-only",
        skip: globalSettings.DISABLE_AUTH || !isMaterialsMandatory,
    })

    const hasAcknowledgedSupportMaterials =
        !isMaterialsMandatory ||
        (dataMaterialsAcknowledgment?.get_support_materials_acknowledgment?.document_ids.length ??
            0) > 0

    const {
        data: castVotes,
        error: errorCastVote,
        startPolling: startCastVotePolling,
        stopPolling: stopCastVotePolling,
    } = useQuery<GetCastVotesQuery>(GET_CAST_VOTES, {
        skip: globalSettings.DISABLE_AUTH,
    })

    const materialsPath = `/tenant/${tenantId}/event/${eventId}/materials${location.search}`

    const handleNavigateMaterials = () => {
        navigate(materialsPath)
    }

    const hasNoElections = !loadingElections && dataElections?.sequent_backend_election.length === 0
    const isPublished = useMemo(
        () => !!dataElectionEvent?.sequent_backend_election_event[0].status?.is_published,
        [dataElectionEvent?.sequent_backend_election_event]
    )

    useEffect(() => {
        if (!dataMaterials || globalSettings.DISABLE_AUTH || !isMaterialsVisible) {
            return
        }

        for (let material of dataMaterials.sequent_backend_support_material) {
            dispatch(setSupportMaterial(material))
        }
    }, [dataMaterials, globalSettings.DISABLE_AUTH, isMaterialsVisible])

    // Errors handling
    useEffect(() => {
        if (globalSettings.DISABLE_AUTH) {
            return
        }
        if (errorElections || errorElectionEvent || errorBallotStyles || errorCastVote) {
            if (errorBallotStyles?.message.includes("x-hasura-area-id")) {
                setErrorMsg(ElectionScreenErrorType.NO_AREA)
            } else if (
                errorElections?.networkError ||
                errorElectionEvent?.networkError ||
                errorBallotStyles?.networkError ||
                errorCastVote?.networkError
            ) {
                setErrorMsg(ElectionScreenErrorType.NETWORK)
            } else {
                setErrorMsg(ElectionScreenErrorType.FETCH_DATA)
            }
        } else if (dataElectionEvent?.sequent_backend_election_event.length === 0) {
            setErrorMsg(ElectionScreenErrorType.NO_ELECTION_EVENT)
        } else if (!isPublished) {
            setAlertMsg(ElectionScreenMsgType.NOT_PUBLISHED)
        } else if (hasNoElections) {
            if (electionIds.length > 0) {
                setErrorMsg(ElectionScreenErrorType.OBTAINING_ELECTION)
                setErrorMsgElectionIds(JSON.stringify(electionIds))
            } else {
                setAlertMsg(ElectionScreenMsgType.NO_ELECTIONS)
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
                setBallotStyleConfigurationError(undefined)
            } catch (error: unknown) {
                if (error instanceof BallotStyleConfigurationError) {
                    setBallotStyleConfigurationError({
                        translationKey: error.translationKey,
                        translationParams: error.translationParams,
                    })
                    setErrorMsg(undefined)
                } else {
                    setBallotStyleConfigurationError(undefined)
                    setErrorMsg(ElectionScreenErrorType.BALLOT_STYLES_EML)
                }
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
                        alias: election.presentation
                            ? translateFromPresentation(
                                  election.presentation,
                                  "alias",
                                  i18n.language,
                                  {
                                      defaultLanguageCode:
                                          election.presentation.language_conf
                                              ?.default_language_code ?? eventDefaultLanguageCode,
                                  }
                              )
                            : undefined,
                    })
                )
            }

            let foundTestElection = dataElections.sequent_backend_election.find((election) => {
                const name = election.presentation
                    ? translateFromPresentation(election.presentation, "name", i18n.language, {
                          defaultLanguageCode:
                              election.presentation.language_conf?.default_language_code ??
                              eventDefaultLanguageCode,
                      })
                    : undefined
                return name?.includes("TEST") ?? false
            })

            if (foundTestElection) {
                setCanVoteTest(false)
            }

            setTestElectionId(foundTestElection?.id || null)
        }
    }, [dataElections, dispatch, eventDefaultLanguageCode, i18n.language])

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
        setMaterialsPolicy(
            getEffectiveSupportMaterialsPolicy(electionEvent?.presentation?.materials)
        )
    }, [electionEvent?.presentation?.materials])

    useEffect(() => {
        if (castVotes?.sequent_backend_cast_vote) {
            const castVoteList = castVotes.sequent_backend_cast_vote
            dispatch(addCastVotes(castVoteList))

            const hasUnresolvedCastVotes = castVoteList.some(
                (castVote) => castVote.status === CastVoteStatus.IN_PROGRESS
            )
            if (hasUnresolvedCastVotes) {
                startCastVotePolling(globalSettings.QUERY_POLL_INTERVAL_MS)
            } else {
                stopCastVotePolling()
            }
        }
    }, [
        castVotes,
        dispatch,
        globalSettings.QUERY_POLL_INTERVAL_MS,
        startCastVotePolling,
        stopCastVotePolling,
    ])

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

    const warningMsg = errorMsg
        ? t(`electionSelectionScreen.errors.${errorMsg}`, {
              electionIds: errorMsgElectionIds,
          })
        : ballotStyleConfigurationError
          ? t(
                ballotStyleConfigurationError.translationKey,
                ballotStyleConfigurationError.translationParams
            )
          : alertMsg
            ? t(`electionSelectionScreen.alerts.${alertMsg}`)
            : undefined

    const showMaterialsGateBanner = isMaterialsMandatory && !hasAcknowledgedSupportMaterials

    if (loadingElectionEvent || loadingElections || loadingBallotStyles) return <CircularProgress />

    return (
        <PageLimit maxWidth="lg" className="election-selection-screen screen">
            <Box marginTop="48px">
                <Stepper selected={0} />
            </Box>

            <TitleSection className="title-section">
                <Box sx={{flex: 1, minWidth: 0}} className="election-selection-heading">
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
                    {warningMsg ? (
                        <Alert severity="warning">{warningMsg}</Alert>
                    ) : (
                        <Typography
                            variant="body1"
                            sx={{color: theme.palette.customGrey.contrastText}}
                        >
                            {stringToHtml(t("electionSelectionScreen.description"))}
                        </Typography>
                    )}
                </Box>
                <PageActions className="election-event-actions">
                    {eventResultsUrl ? (
                        <Button
                            className="results-button election-event-results-button"
                            variant="secondary"
                            component="a"
                            href={eventResultsUrl}
                            target="_blank"
                            rel="noreferrer"
                        >
                            {t("electionSelectionScreen.resultsButton")}
                        </Button>
                    ) : null}
                    {isMaterialsVisible && electionEvent ? (
                        <Button onClick={handleNavigateMaterials}>
                            {translateFromPresentation(
                                electionEvent,
                                "materialsTitle",
                                i18n.language,
                                {defaultLanguageCode: eventDefaultLanguageCode}
                            ) || t("materials.common.label")}
                        </Button>
                    ) : null}
                </PageActions>
            </TitleSection>
            {showMaterialsGateBanner ? (
                <Alert
                    severity="warning"
                    className="materials-gate-banner"
                    sx={{marginBottom: "16px"}}
                >
                    <Trans
                        i18nKey="electionSelectionScreen.materialsGate.instructions"
                        components={{
                            MaterialsLink: <MaterialsGateLink to={materialsPath} />,
                        }}
                    />
                </Alert>
            ) : null}
            <ElectionContainer className="elections-list">
                {!hasNoElections ? (
                    electionIds.map((electionId) => (
                        <ElectionWrapper
                            electionId={electionId}
                            key={electionId}
                            bypassChooser={bypassChooser}
                            canVoteTest={canVoteTest}
                            materialsGate={showMaterialsGateBanner}
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
