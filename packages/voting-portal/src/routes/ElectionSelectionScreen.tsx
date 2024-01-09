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
import {CustomError} from "./ErrorPage"

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
}

const ElectionWrapper: React.FC<ElectionWrapperProps> = ({electionId}) => {
    const election = useAppSelector(selectElectionById(electionId))
    const castVotes = useAppSelector(selectCastVotesByElectionId(String(electionId)))
    const navigate = useNavigate()
    const {i18n} = useTranslation()

    if (!election) {
        throw new CustomError("Internal Error")
    }

    const canVote = castVotes.length < (election?.num_allowed_revotes ?? 1)

    const onClickToVote = () => {
        navigate(`../election/${electionId}/start`)
    }

    const handleClickBallotLocator = () => {
        navigate(`../election/${electionId}/ballot-locator`)
    }

    return (
        <SelectElection
            isActive={true}
            isOpen={true}
            canVote={canVote}
            title={translateElection(election, "name", i18n.language) || ""}
            electionHomeUrl={"https://sequentech.io"}
            hasVoted={castVotes.length > 0}
            onClickToVote={onClickToVote}
            onClickElectionResults={() => undefined}
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
            console.log(`Error loading fake EML: ${error}`)
            console.log(election)
            throw new CustomError("Error loading fake ballot style and election")
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

const convertToElection = (input: IElectionDTO): IElection => ({
    id: input.id,
    annotations: null,
    created_at: null,
    dates: null,
    description: input.description,
    election_event_id: input.id,
    eml: JSON.stringify(input),
    is_consolidated_ballot_encoding: false,
    labels: null,
    last_updated_at: null,
    name: input.description,
    num_allowed_revotes: 1,
    presentation: null,
    spoil_ballot_option: true,
    status: "OPEN",
    tenant_id: input.id,
})

export const ElectionSelectionScreen: React.FC = () => {
    const {t} = useTranslation()
    const navigate = useNavigate()

    const {globalSettings} = useContext(SettingsContext)
    const {eventId, tenantId} = useParams<{eventId?: string; tenantId?: string}>()

    const ballotStyleElectionIds = useAppSelector(selectBallotStyleElectionIds)
    const electionIds = useAppSelector(selectElectionIds)
    const dispatch = useAppDispatch()

    const [openChooserHelp, setOpenChooserHelp] = useState(false)
    const [isMaterialsActivated, setIsMaterialsActivated] = useState<boolean>(false)

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

    const {data: castVotes} = useQuery<GetCastVotesQuery>(GET_CAST_VOTES)

    const hasNoResults = electionIds.length === 0

    const handleNavigateMaterials = () => {
        navigate(`/tenant/${tenantId}/event/${eventId}/materials`)
    }

    useEffect(() => {
        if (errorBallotStyles || errorElections || errorElectionEvent) {
            throw new CustomError("Unable to fetch data")
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
        if (dataElectionEvent && dataElectionEvent.sequent_backend_election_event.length > 0) {
            const record = dataElectionEvent?.sequent_backend_election_event?.[0]

            setIsMaterialsActivated(record?.presentation?.materials?.activated || false)
        }
    }, [dataElectionEvent])

    useEffect(() => {
        const record = dataElectionEvent?.sequent_backend_election_event?.[0] ?? null

        if (!record?.status) {
            return
        }

        const status = record.status as IElectionEventStatus
        console.log("LS -> src/routes/ElectionSelectionScreen.tsx:278 -> status: ", status)
    }, [dataElectionEvent])

    useEffect(() => {
        if (!castVotes?.sequent_backend_cast_vote) {
            return
        }

        dispatch(addCastVotes(castVotes.sequent_backend_cast_vote))
    }, [castVotes, dispatch])

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
            <ElectionContainer>
                {!hasNoResults ? (
                    electionIds.map((electionId) => (
                        <ElectionWrapper electionId={electionId} key={electionId} />
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
