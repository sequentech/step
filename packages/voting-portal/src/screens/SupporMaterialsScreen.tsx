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
    translate,
    translateElection,
} from "@sequentech/ui-essentials"
import {faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import {styled} from "@mui/material/styles"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {IBallotStyle, setBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import {useNavigate, useParams} from "react-router-dom"
import {useQuery} from "@apollo/client"
import {GET_BALLOT_STYLES} from "../queries/GetBallotStyles"
import {GetBallotStylesQuery, GetElectionsQuery, Sequent_Backend_Area} from "../gql/graphql"
import {IBallotStyle as IElectionDTO} from "sequent-core"
import {resetBallotSelection} from "../store/ballotSelections/ballotSelectionsSlice"
import {IElection, selectElectionById, setElection} from "../store/elections/electionsSlice"
import {GET_ELECTIONS} from "../queries/GetElections"
import {AppDispatch} from "../store/store"
import {ELECTIONS_LIST} from "../fixtures/election"
import {TenantEventContext} from ".."
import {AuthContext} from "../providers/AuthContextProvider"
import {SettingsContext} from "../providers/SettingsContextProvider"
import ChevronLeftIcon from "@mui/icons-material/ChevronLeft"
import {GET_ELECTION_EVENT} from "../queries/GetElectionEvent"
import {GET_SUPPORT_MATERIALS} from "../queries/GetSupportMaterials"
import { SupportMatherial } from '../components/SupportMatherial/SupportMatherial'

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
    material: any
}

const ElectionWrapper: React.FC<ElectionWrapperProps> = ({material}) => {
    console.log("ElectionWrapper", material)

    // const election = useAppSelector(selectElectionById(electionId))
    const {tenantId, eventId} = useContext(TenantEventContext)
    const navigate = useNavigate()
    const {i18n} = useTranslation()

    const onClickToVote = () => {
        console.log("onClickToVote")
    }

    return (
        <SupportMatherial
            title={translate(material.data, "title", i18n.language) || ""}
            subtitle={translate(material.data, "subtitle", i18n.language) || ""}
            kind={material.kind || ""}
            onClickToVote={onClickToVote}
            onClickElectionResults={() => undefined}
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
            dispatch(setBallotStyle(formattedBallotStyle))
            dispatch(
                resetBallotSelection({
                    ballotStyle: formattedBallotStyle,
                })
            )
        } catch (error) {
            console.log(`Error loading fake EML: ${error}`)
            console.log(election)
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
                })
            )
        } catch (error) {
            console.log(`Error loading EML: ${error}`)
            console.log(ballotEml)
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

export const SupportMaterialsScreen: React.FC = () => {
    const [ballotStyleElectionIds, setBallotStyleElectionIds] = useState<Array<string>>([])
    const {loading, error, data} = useQuery<GetBallotStylesQuery>(GET_BALLOT_STYLES)
    const {
        loading: loadingElections,
        error: errorElections,
        data: dataElections,
    } = useQuery<GetElectionsQuery>(GET_ELECTIONS, {
        variables: {
            electionIds: ballotStyleElectionIds,
        },
    })
    const dispatch = useAppDispatch()
    const {globalSettings} = useContext(SettingsContext)
    const {t, i18n} = useTranslation()
    const [openChooserHelp, setOpenChooserHelp] = useState(false)
    const navigate = useNavigate()
    const {eventId, tenantId} = useParams<{eventId?: string; tenantId?: string}>()

    const {
        loading: loadingMaterials,
        error: errorMaterials,
        data: dataMaterials,
    } = useQuery<any>(GET_SUPPORT_MATERIALS, {
        variables: {
            electionEventId: eventId || "",
            tenantId: tenantId || "",
        },
    })

    console.log("eventId", eventId)
    console.log("tenantId", tenantId)
    console.log("dataMaterials", dataMaterials?.sequent_backend_support_material)
    console.log("errorMaterials", errorMaterials)
    console.log("loadingMaterials", loadingMaterials)

    const {
        loading: loadingElectionEvent,
        error: errorElectionEvent,
        data: dataElectionEvent,
    } = useQuery<any>(GET_ELECTION_EVENT, {
        variables: {
            electionEventId: eventId,
            tenantId,
        },
    })

    const [electionIds, setElectionIds] = useState<Array<string>>([])

    useEffect(() => {
        if (!loadingElections && !errorElections && dataElections) {
            setElectionIds(dataElections.sequent_backend_election.map((election) => election.id))
            for (let election of dataElections.sequent_backend_election) {
                dispatch(setElection(election))
            }
        }
    }, [loadingElections, errorElections, dataElections, dispatch])

    useEffect(() => {
        if (!loading && !error && data) {
            updateBallotStyleAndSelection(data, dispatch)

            let electionIds = data.sequent_backend_ballot_style
                .map((ballotStyle) => ballotStyle.election_id as string | null)
                .filter((ballotStyle) => isString(ballotStyle)) as Array<string>

            setBallotStyleElectionIds(electionIds)
        }
    }, [loading, error, data, dispatch])

    useEffect(() => {
        console.log("i18n.language", i18n.language)
    }, [i18n.language])

    useEffect(() => {
        if (globalSettings.DISABLE_AUTH) {
            setElectionIds(ELECTIONS_LIST.map((election) => election.id))
        }

        for (let election of ELECTIONS_LIST) {
            dispatch(setElection(convertToElection(election)))
            fakeUpdateBallotStyleAndSelection(dispatch)
        }
    }, [])

    const [materialsTitles, setMaterialsTitles] = useState<any>({})

    useEffect(() => {
        if (dataElectionEvent && dataElectionEvent.sequent_backend_election_event.length > 0) {
            console.log(
                "dataElectionEvent",
                dataElectionEvent?.sequent_backend_election_event?.[0]?.presentation
            )
            setMaterialsTitles(dataElectionEvent?.sequent_backend_election_event?.[0])
        }
    }, [dataElectionEvent])

    const handleNavigateMaterials = () => {
        navigate(`/tenant/${tenantId}/event/${eventId}/election-chooser`)
    }

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
                        <Box>
                            {translateElection(materialsTitles, "materialsTitle", i18n.language)}
                        </Box>
                    </StyledTitle>
                    <Typography variant="body1" sx={{color: theme.palette.customGrey.contrastText}}>
                        {stringToHtml(
                            translateElection(materialsTitles, "materialsSubtitle", i18n.language)
                        )}
                    </Typography>
                </Box>
                <Button startIcon={<ChevronLeftIcon />} onClick={handleNavigateMaterials}>
                    {t("materials.common.back")}
                </Button>
            </Box>
            <ElectionContainer>
                {dataMaterials?.sequent_backend_support_material?.map((material: any) => (
                    <ElectionWrapper material={material} key={material.id} />
                ))}
            </ElectionContainer>
        </PageLimit>
    )
}
