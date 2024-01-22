// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, Button, Typography} from "@mui/material"
import React, {useContext, useEffect, useState} from "react"
import {useTranslation} from "react-i18next"
import {
    BreadCrumbSteps,
    Dialog,
    IconButton,
    PageLimit,
    SelectElection,
    isString,
    stringToHtml,
    theme,
    translateElection,
    EVotingStatus,
    IElectionEventStatus,
    isUndefined,
} from "@sequentech/ui-essentials"
import {faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import {styled} from "@mui/material/styles"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {
    IBallotStyle,
    selectBallotStyleElectionIds,
    setBallotStyle,
} from "../store/ballotStyles/ballotStylesSlice"
import {resetBallotSelection} from "../store/ballotSelections/ballotSelectionsSlice"
import {
    IElection,
    selectElectionById,
    setElection,
    selectElectionIds,
} from "../store/elections/electionsSlice"
import {AppDispatch} from "../store/store"
import {addCastVotes, selectCastVotesByElectionId} from "../store/castVotes/castVotesSlice"
import {useNavigate, useParams} from "react-router-dom"
import {useQuery} from "@apollo/client"
import {GET_BALLOT_STYLES} from "../queries/GetBallotStyles"
import {
    GetBallotStylesQuery,
    GetCastVotesQuery,
    GetElectionEventQuery,
    GetElectionsQuery,
} from "../gql/graphql"
import {IBallotStyle as IElectionDTO} from "sequent-core"
import {GET_ELECTIONS} from "../queries/GetElections"
import {ELECTIONS_LIST} from "../fixtures/election"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {GET_ELECTION_EVENT} from "../queries/GetElectionEvent"
import {GET_CAST_VOTES} from "../queries/GetCastVotes"
import {VotingPortalError, VotingPortalErrorType} from "../services/VotingPortalError"
import {
    selectElectionEventById,
    setElectionEvent,
} from "../store/electionEvents/electionEventsSlice"
import {TenantEventType} from ".."

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
}

const ElectionWrapper: React.FC<ElectionWrapperProps> = ({electionId, bypassChooser}) => {
    const navigate = useNavigate()
    const {i18n} = useTranslation()

    const {eventId} = useParams<TenantEventType>()
    const election = useAppSelector(selectElectionById(electionId))
    const castVotes = useAppSelector(selectCastVotesByElectionId(String(electionId)))
    const electionEvent = useAppSelector(selectElectionEventById(eventId))

    if (!election) {
        throw new VotingPortalError(VotingPortalErrorType.INTERNAL_ERROR)
    }

    const eventStatus = electionEvent?.status as IElectionEventStatus | null
    const isVotingOpen = eventStatus?.voting_status === EVotingStatus.OPEN
    const canVote = castVotes.length < (election?.num_allowed_revotes ?? 1) && isVotingOpen

    const onClickToVote = () => {
        if (!canVote) {
            return
        }
        navigate(`../election/${electionId}/start`)
    }

    const handleClickBallotLocator = () => {
        navigate(`../election/${electionId}/ballot-locator`)
    }

    if (bypassChooser) {
        onClickToVote()
    }

    return (
        <SelectElection
            isActive={canVote}
            isOpen={isVotingOpen}
            title={translateElection(election, "name", i18n.language) || ""}
            electionHomeUrl={"https://sequentech.io"}
            hasVoted={castVotes.length > 0}
            onClickToVote={canVote ? onClickToVote : undefined}
            onClickBallotLocator={handleClickBallotLocator}
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
            dispatch(setElection(election))
            dispatch(setBallotStyle(formattedBallotStyle))
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

const updateBallotStyleAndSelection = (data: GetBallotStylesQuery, dispatch: AppDispatch) => {
    for (let ballotStyle of data.sequent_backend_ballot_style) {
        const ballotEml = ballotStyle.ballot_eml
        if (!isString(ballotEml)) {
            continue
        }
        try {
            const electionData: IElectionDTO = JSON.parse(ballotEml)
            const formattedBallotStyle: IBallotStyle = {
                id: ballotStyle.id,
                election_id: ballotStyle.election_id,
                election_event_id: ballotStyle.election_event_id,
                tenant_id: ballotStyle.tenant_id,
                ballot_eml: electionData,
                ballot_signature: ballotStyle.ballot_signature,
                created_at: ballotStyle.created_at,
                area_id: ballotStyle.area_id,
                annotations: ballotStyle.annotations,
                labels: ballotStyle.labels,
                last_updated_at: ballotStyle.last_updated_at,
            }
            dispatch(setBallotStyle(formattedBallotStyle))
            dispatch(
                resetBallotSelection({
                    ballotStyle: formattedBallotStyle,
                    force: true,
                })
            )
        } catch (error) {
            console.log(`Error loading EML: ${error}`)
            console.log(ballotEml)
            throw error
        }
    }
}

export const ElectionSelectionScreen: React.FC = () => {
    const {t} = useTranslation()
    const navigate = useNavigate()

    const {globalSettings} = useContext(SettingsContext)
    const {eventId, tenantId} = useParams<{eventId?: string; tenantId?: string}>()

    const ballotStyleElectionIds = useAppSelector(selectBallotStyleElectionIds)
    const electionIds = useAppSelector(selectElectionIds)
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const dispatch = useAppDispatch()

    const [openChooserHelp, setOpenChooserHelp] = useState(false)
    const [isMaterialsActivated, setIsMaterialsActivated] = useState<boolean>(false)
    const [bypassChooser, setBypassChooser] = useState<boolean>(false)

    const {error: errorBallotStyles, data: dataBallotStyles} =
        useQuery<GetBallotStylesQuery>(GET_BALLOT_STYLES)

    const {error: errorElections, data: dataElections} = useQuery<GetElectionsQuery>(
        GET_ELECTIONS,
        {
            variables: {
                electionIds: ballotStyleElectionIds,
            },
        }
    )

    const {error: errorElectionEvent, data: dataElectionEvent} = useQuery<GetElectionEventQuery>(
        GET_ELECTION_EVENT,
        {
            variables: {
                electionEventId: eventId,
                tenantId,
            },
        }
    )

    const {data: castVotes, error: errorCastVote} = useQuery<GetCastVotesQuery>(GET_CAST_VOTES)

    const hasNoResults = electionIds.length === 0

    const handleNavigateMaterials = () => {
        navigate(`/tenant/${tenantId}/event/${eventId}/materials`)
    }

    useEffect(() => {
        if (errorBallotStyles || errorElections || errorElectionEvent) {
            throw new VotingPortalError(VotingPortalErrorType.UNABLE_TO_FETCH_DATA)
        }
    }, [errorElections, errorBallotStyles, errorElectionEvent])

    useEffect(() => {
        if (dataBallotStyles && dataBallotStyles.sequent_backend_ballot_style.length > 0) {
            updateBallotStyleAndSelection(dataBallotStyles, dispatch)
        } else if (globalSettings.DISABLE_AUTH) {
            fakeUpdateBallotStyleAndSelection(dispatch)
        }
    }, [globalSettings.DISABLE_AUTH, dataBallotStyles, dispatch])

    useEffect(() => {
        if (dataElections && dataElections.sequent_backend_election.length > 0) {
            for (let election of dataElections.sequent_backend_election) {
                dispatch(setElection(election))
            }
        }
    }, [dataElections, dispatch])

    useEffect(() => {
        const record = dataElectionEvent?.sequent_backend_election_event?.[0]

        if (record) {
            dispatch(setElectionEvent(record))
            setIsMaterialsActivated(record?.presentation?.materials?.activated || false)
        }
    }, [dataElectionEvent, dispatch])

    useEffect(() => {
        if (castVotes?.sequent_backend_cast_vote) {
            dispatch(addCastVotes(castVotes.sequent_backend_cast_vote))
        }
    }, [castVotes, dispatch])

    useEffect(() => {
        const newBypassChooser =
            1 === electionIds.length &&
            !errorCastVote &&
            !isUndefined(castVotes) &&
            !!electionEvent &&
            !!dataElections
        if (newBypassChooser && !bypassChooser) {
            setBypassChooser(newBypassChooser)
            const electionId = electionIds[0]

            navigate(`../election/${electionId}/start`)
        }
    }, [castVotes, electionIds, errorCastVote, castVotes, electionEvent, dataElections])

    return (
        <PageLimit maxWidth="lg">
            <Box marginTop="48px">
                <BreadCrumbSteps
                    labels={[
                        "breadcrumbSteps.electionList",
                        "breadcrumbSteps.ballot",
                        "breadcrumbSteps.review",
                        "breadcrumbSteps.confirmation",
                    ]}
                    selected={0}
                />
            </Box>
            <Box
                sx={{
                    display: "flex",
                    flexDirection: "row",
                    justifyContent: "space-between",
                    alignItems: "center",
                    minHeight: "100px",
                }}
            >
                <Box>
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
                    <Typography variant="body1" sx={{color: theme.palette.customGrey.contrastText}}>
                        {stringToHtml(t("electionSelectionScreen.description"))}
                    </Typography>
                </Box>
                {isMaterialsActivated ? (
                    <Button onClick={handleNavigateMaterials}>{t("materials.common.label")}</Button>
                ) : null}
            </Box>
            <ElectionContainer className="elections-list">
                {!hasNoResults ? (
                    electionIds.map((electionId) => (
                        <ElectionWrapper
                            electionId={electionId}
                            key={electionId}
                            bypassChooser={bypassChooser}
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
