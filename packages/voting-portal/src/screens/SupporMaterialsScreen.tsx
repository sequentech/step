// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Button, Typography} from "@mui/material"
import React, {useContext, useEffect, useState} from "react"
import {useTranslation} from "react-i18next"
import {
    BreadCrumbSteps,
    PageLimit,
    isString,
    stringToHtml,
    theme,
    translate,
    translateElection,
} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"
import {useAppDispatch} from "../store/hooks"
import {IBallotStyle, setBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import {useNavigate, useParams} from "react-router-dom"
import {useQuery} from "@apollo/client"
import {GET_BALLOT_STYLES} from "../queries/GetBallotStyles"
import {
    GetBallotStylesQuery,
    GetElectionEventQuery,
    GetElectionsQuery,
    Sequent_Backend_Area,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
} from "../gql/graphql"
import {IBallotStyle as IElectionDTO} from "sequent-core"
import {resetBallotSelection} from "../store/ballotSelections/ballotSelectionsSlice"
import {IElection, setElection} from "../store/elections/electionsSlice"
import {GET_ELECTIONS} from "../queries/GetElections"
import {AppDispatch} from "../store/store"
import {ELECTIONS_LIST} from "../fixtures/election"
import {TenantEventContext} from ".."
import {SettingsContext} from "../providers/SettingsContextProvider"
import ChevronLeftIcon from "@mui/icons-material/ChevronLeft"
import {GET_ELECTION_EVENT} from "../queries/GetElectionEvent"
import {GET_SUPPORT_MATERIALS} from "../queries/GetSupportMaterials"
import {SupportMaterial} from "../components/SupportMaterial/SupportMaterial"

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
    // const election = useAppSelector(selectElectionById(electionId))
    const {tenantId} = useContext(TenantEventContext)
    const {i18n} = useTranslation()

    return (
        <SupportMaterial
            title={translate(material.data, "title", i18n.language) || ""}
            subtitle={translate(material.data, "subtitle", i18n.language) || ""}
            kind={material.kind || ""}
            tenantId={tenantId || ""}
            documentId={material.document_id || ""}
        />
    )
}

export const SupportMaterialsScreen: React.FC = () => {
    const {t, i18n} = useTranslation()
    const navigate = useNavigate()
    const {eventId, tenantId} = useParams<{eventId?: string; tenantId?: string}>()

    const {data: dataMaterials} = useQuery<any>(
        GET_SUPPORT_MATERIALS,
        {
            variables: {
                electionEventId: eventId || "",
                tenantId: tenantId || "",
            },
        }
    )

    const {data: dataElectionEvent} = useQuery<GetElectionEventQuery>(GET_ELECTION_EVENT, {

        variables: {
            electionEventId: eventId,
            tenantId,
        },
    })

    const [materialsTitles, setMaterialsTitles] = useState<any>({})

    useEffect(() => {
        if (dataElectionEvent && dataElectionEvent.sequent_backend_election_event.length > 0) {
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
