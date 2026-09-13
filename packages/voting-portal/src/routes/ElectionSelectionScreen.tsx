// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, Button, CircularProgress, Typography, Alert} from "@mui/material"
import React, {useContext, useEffect, useState} from "react"
import {Trans, useTranslation} from "react-i18next"
import {Dialog, IconButton, PageLimit, SelectElection, theme} from "@sequentech/ui-essentials"
import {
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
import {selectBallotStyleByElectionId} from "../store/ballotStyles/ballotStylesSlice"
import {selectElectionById, setElection, selectElectionIds} from "../store/elections/electionsSlice"
import {
    addCastVotes,
    parseCastVoteStatus,
    CastVoteStatus,
    selectCastVotesByElectionId,
} from "../store/castVotes/castVotesSlice"
import {Link as RouterLink, useLocation, useNavigate, useParams} from "react-router-dom"
import {useQuery} from "@apollo/client/react"
import {isApolloTransportError} from "../services/ApolloErrors"
import {useVoterContext} from "../hooks/useVoterContext"
import {isElectionOpenForVoting} from "../services/VotingAvailability"
import {
    GetCastVotesQuery,
    GetSupportMaterialsQuery,
    GetSupportMaterialsAcknowledgmentQuery,
} from "../gql/graphql"
import {SettingsContext} from "../providers/SettingsContextProvider"
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
import {
    isAcclaimedElectionCompleted,
    selectBypassChooser,
    setBypassChooser,
} from "../store/extra/extraSlice"
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
    summary?: import("../services/PublishedBallots").BallotSummary

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
    summary,
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
    const isAcclaimedCompleted = useAppSelector(isAcclaimedElectionCompleted(electionId))
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
    const isVotingOpen = () =>
        isElectionOpenForVoting({
            electionStatus,
            eventStatus: electionEvent?.status as IElectionEventStatus | null,
            channels: election.voting_channels,
            areaPresentation:
                summary?.area_presentation ?? ballotStyle?.ballot_eml.area_presentation,
            isKiosk,
        })

    const isEarlyVotingPolicyEnabled = () => {
        let area_presentation = (summary?.area_presentation ??
            ballotStyle?.ballot_eml?.area_presentation) as IAreaPresentation | undefined
        return area_presentation?.allow_early_voting === EEarlyVotingPolicy.ALLOW_EARLY_VOTING
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

        if (isAcclaimedCompleted) {
            return false
        }

        if (election.num_allowed_revotes === 0) {
            return true
        }

        return (
            isPreview || (castVotes.length < (election.num_allowed_revotes ?? 1) && isVotingOpen())
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
        if (bypassChooser && election) {
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
            electionDates={summary?.election_dates ?? ballotStyle?.ballot_eml?.election_dates}
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

const ElectionSelectionScreen: React.FC = () => {
    const {t, i18n} = useTranslation()
    const navigate = useNavigate()
    const location = useLocation()

    const {globalSettings} = useContext(SettingsContext)
    const {eventId, tenantId} = useParams<{eventId?: string; tenantId?: string}>()
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const eventDefaultLanguageCode =
        electionEvent?.presentation?.language_conf?.default_language_code
    const electionIds = useAppSelector(selectElectionIds)
    const dispatch = useAppDispatch()
    const [canVoteTest, setCanVoteTest] = useState<boolean>(true)
    const [testElectionId, setTestElectionId] = useState<string | null>(null)
    const castVotesTestElection = useAppSelector(
        selectCastVotesByElectionId(String(testElectionId || tenantId))
    )
    const [openChooserHelp, setOpenChooserHelp] = useState(false)
    // Presentation comes from the immutable S3 publication snapshot.
    const materialsPolicy = getEffectiveSupportMaterialsPolicy(
        electionEvent?.presentation?.materials
    )
    const isMaterialsVisible = materialsPolicy !== ESupportMaterialsPolicy.OFF
    const isMaterialsMandatory = materialsPolicy === ESupportMaterialsPolicy.MANDATORY_FOR_VOTING
    const bypassChooser = useAppSelector(selectBypassChooser())
    const eventResultsUrl =
        globalSettings.RESULTS_PORTAL_URL &&
        eventId &&
        isResultsWebsiteEnabled(electionEvent) &&
        isElectionEventVotingClosed(electionEvent)
            ? `${globalSettings.RESULTS_PORTAL_URL.replace(/\/+$/, "")}/${eventId}`
            : undefined

    const {data, error, loading, summaries} = useVoterContext()

    // Materials
    const {data: dataMaterials} = useQuery<GetSupportMaterialsQuery>(GET_SUPPORT_MATERIALS, {
        variables: {
            electionEventId: eventId || "",
            tenantId: tenantId || "",
        },
        skip: globalSettings.DISABLE_AUTH || !isMaterialsVisible, // Skip query if in demo mode
    })

    const {data: dataMaterialsAcknowledgment} = useQuery<GetSupportMaterialsAcknowledgmentQuery>(
        GET_SUPPORT_MATERIALS_ACKNOWLEDGMENT,
        {
            variables: {
                electionEventId: eventId || "",
            },
            // Support Materials writes the acknowledgment straight into the Apollo
            // cache on Continue (see SupportMaterialsScreen), so the default
            // cache-first policy already reflects it instantly on return here
            // instead of re-gating the Ballot list behind a fresh network round trip.
            skip: globalSettings.DISABLE_AUTH || !isMaterialsMandatory,
        }
    )

    // Whether we have a definitive answer yet. On a fresh page load the Apollo
    // cache starts empty, so these queries are genuinely loading for a moment -
    // default to "not yet known" rather than "not acknowledged" so voting
    // stays blocked (safe) without flashing the gate banner (misleading) for a
    // voter who has, in fact, already acknowledged.
    const hasAcknowledgmentLoaded =
        !isMaterialsMandatory ||
        (dataMaterialsAcknowledgment !== undefined && dataMaterials !== undefined)

    const acknowledgedDocumentIds = new Set(
        dataMaterialsAcknowledgment?.get_support_materials_acknowledgment?.document_ids ?? []
    )

    // Acknowledging is per-document, so a material published after the
    // voter's last acknowledgment must gate voting again - checking the
    // acknowledged count alone would let that new material slip through.
    const hasAcknowledgedSupportMaterials =
        !isMaterialsMandatory ||
        (dataMaterials?.sequent_backend_support_material.every(
            (material) => !material.document_id || acknowledgedDocumentIds.has(material.document_id)
        ) ??
            false)

    const hasPendingCastVotes = useAppSelector((state) =>
        Object.values(state.castVotes).some((votes) =>
            // The cast action returns no status. Refresh that record before
            // deciding whether it was accepted or discarded asynchronously.
            votes.some((vote) => vote.status == null || vote.status === CastVoteStatus.IN_PROGRESS)
        )
    )
    const {
        data: polledCastVotes,
        error: errorCastVote,
        startPolling: startCastVotePolling,
        stopPolling: stopCastVotePolling,
    } = useQuery<GetCastVotesQuery>(GET_CAST_VOTES, {
        // The bootstrap supplies the initial cast metadata. Only unresolved
        // casts need the existing narrow polling query.
        fetchPolicy: "network-only",
        skip:
            globalSettings.DISABLE_AUTH ||
            (!hasPendingCastVotes &&
                !data?.sequent_backend_cast_vote.some(
                    (vote) => vote.status === CastVoteStatus.IN_PROGRESS
                )),
    })
    const castVotes = polledCastVotes ?? data

    const materialsPath = `/tenant/${tenantId}/event/${eventId}/materials${location.search}`
    const materialsTitle =
        (electionEvent &&
            translateFromPresentation(electionEvent, "materialsTitle", i18n.language, {
                defaultLanguageCode: eventDefaultLanguageCode,
            })) ||
        t("materials.common.label")

    const handleNavigateMaterials = () => {
        navigate(materialsPath)
    }

    const hasNoElections = !loading && data?.sequent_backend_election.length === 0
    const isPublished = !!data?.sequent_backend_election_event[0]?.status?.is_published

    useEffect(() => {
        if (!dataMaterials || globalSettings.DISABLE_AUTH || !isMaterialsVisible) {
            return
        }

        for (let material of dataMaterials.sequent_backend_support_material) {
            dispatch(setSupportMaterial(material))
        }
    }, [dataMaterials, globalSettings.DISABLE_AUTH, isMaterialsVisible])

    useEffect(() => {
        if (data && data.sequent_backend_election.length > 0) {
            for (let election of data.sequent_backend_election) {
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

            let foundTestElection = data.sequent_backend_election.find((election) => {
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
    }, [data, dispatch, eventDefaultLanguageCode, i18n.language])

    useEffect(() => {
        if (!testElectionId) {
            return
        }
        setCanVoteTest(castVotesTestElection.length > 0)
    }, [castVotesTestElection, testElectionId, setCanVoteTest])

    useEffect(() => {
        const record = data?.sequent_backend_election_event?.[0]
        if (record) {
            dispatch(setElectionEvent(record))
        }
    }, [data, dispatch])

    useEffect(() => {
        if (castVotes?.sequent_backend_cast_vote) {
            const castVoteList = castVotes.sequent_backend_cast_vote
            dispatch(
                addCastVotes(
                    castVoteList.map((vote) => ({
                        ...vote,
                        status: parseCastVoteStatus(vote.status),
                    }))
                )
            )

            const hasUnresolvedCastVotes =
                hasPendingCastVotes ||
                castVoteList.some((castVote) => castVote.status === CastVoteStatus.IN_PROGRESS)
            if (hasUnresolvedCastVotes) {
                startCastVotePolling(globalSettings.QUERY_POLL_INTERVAL_MS)
            } else {
                stopCastVotePolling()
            }
        }
    }, [
        castVotes,
        hasPendingCastVotes,
        dispatch,
        globalSettings.QUERY_POLL_INTERVAL_MS,
        startCastVotePolling,
        stopCastVotePolling,
    ])

    useEffect(() => {
        const skipPolicy = electionEvent?.presentation?.skip_election_list ?? false
        console.log("skipPolicy", skipPolicy)
        const newBypassChooser =
            skipPolicy &&
            1 === electionIds.length &&
            !errorCastVote &&
            !isUndefined(castVotes) &&
            !!electionEvent &&
            !!data

        if (newBypassChooser && !bypassChooser) {
            console.log("new baypass chooser", newBypassChooser)
            dispatch(setBypassChooser(newBypassChooser))
        }
    }, [castVotes, electionIds, errorCastVote, electionEvent, data, bypassChooser, dispatch])

    let warningMsg: string | undefined
    if (!globalSettings.DISABLE_AUTH) {
        let errorType: ElectionScreenErrorType | undefined
        let alertType: ElectionScreenMsgType | undefined
        if (error || errorCastVote) {
            errorType = error?.message.includes("x-hasura-area-id")
                ? ElectionScreenErrorType.NO_AREA
                : isApolloTransportError(error) || isApolloTransportError(errorCastVote)
                  ? ElectionScreenErrorType.NETWORK
                  : ElectionScreenErrorType.FETCH_DATA
        } else if (data?.sequent_backend_election_event.length === 0) {
            errorType = ElectionScreenErrorType.NO_ELECTION_EVENT
        } else if (!isPublished) {
            alertType = ElectionScreenMsgType.NOT_PUBLISHED
        } else if (hasNoElections) {
            if (electionIds.length > 0) errorType = ElectionScreenErrorType.OBTAINING_ELECTION
            else alertType = ElectionScreenMsgType.NO_ELECTIONS
        }
        warningMsg = errorType
            ? t(`electionSelectionScreen.errors.${errorType}`, {
                  electionIds: JSON.stringify(electionIds),
              })
            : alertType
              ? t(`electionSelectionScreen.alerts.${alertType}`)
              : undefined
    }

    // Block voting until we positively know the voter has acknowledged.
    const materialsGate =
        isMaterialsMandatory && !(hasAcknowledgmentLoaded && hasAcknowledgedSupportMaterials)

    // Only show the instruction banner once acknowledgment status is
    // positively known to be missing - never while it's still loading, so a
    // page refresh doesn't flash it before the "already acknowledged" result
    // arrives.
    const showMaterialsGateBanner =
        isMaterialsMandatory && hasAcknowledgmentLoaded && !hasAcknowledgedSupportMaterials

    if (loading)
        return (
            <CircularProgress
                className="election-selection-progress"
                aria-label={t("a11y.loading")}
            />
        )

    return (
        <PageLimit maxWidth="lg" className="election-selection-screen screen">
            <Box className="stepper-box" marginTop="48px">
                <Stepper selected={0} />
            </Box>

            <TitleSection className="title-section">
                <Box sx={{flex: 1, minWidth: 0}} className="election-selection-heading">
                    <StyledTitle className="screen-title" variant="h1">
                        <Box className="screen-title-text">
                            {t("electionSelectionScreen.title")}
                        </Box>
                        <IconButton
                            buttonClassName="screen-help-button"
                            icon={faCircleQuestion}
                            sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                            fontSize="16px"
                            onClick={() => setOpenChooserHelp(true)}
                            ariaLabel={t("a11y.helpAbout", {
                                topic: t("electionSelectionScreen.chooserHelpDialog.title"),
                            })}
                        />
                        <Dialog
                            className="screen-help-dialog election-selection-help-dialog"
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
                        <Alert className="election-selection-warning" severity="warning">
                            {warningMsg}
                        </Alert>
                    ) : (
                        <Typography
                            className="screen-description"
                            variant="body1"
                            component="div"
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
                        <Button
                            className="support-materials-button"
                            onClick={handleNavigateMaterials}
                        >
                            {materialsTitle}
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
                        values={{materialsTitle}}
                        components={{
                            MaterialsLink: (
                                <MaterialsGateLink
                                    className="materials-gate-link"
                                    to={materialsPath}
                                />
                            ),
                        }}
                    />
                </Alert>
            ) : null}
            <ElectionContainer
                className="elections-list"
                role={hasNoElections ? undefined : "list"}
            >
                {!hasNoElections ? (
                    electionIds.map((electionId) => (
                        <ElectionWrapper
                            summary={summaries?.[electionId]}
                            electionId={electionId}
                            key={electionId}
                            bypassChooser={bypassChooser}
                            canVoteTest={canVoteTest}
                            materialsGate={materialsGate}
                        />
                    ))
                ) : (
                    <Box className="elections-empty" sx={{margin: "auto"}}>
                        <Typography className="election-selection-empty">
                            {t("electionSelectionScreen.noResults")}
                        </Typography>
                    </Box>
                )}
            </ElectionContainer>
        </PageLimit>
    )
}

export default ElectionSelectionScreen
