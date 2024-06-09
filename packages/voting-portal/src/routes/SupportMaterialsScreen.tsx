// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, Button, Typography} from "@mui/material"
import React, {useEffect, useState} from "react"
import {useTranslation} from "react-i18next"
import {
    PageLimit,
    stringToHtml,
    theme,
    translate,
    translateElection,
} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"
import {TenantEventType} from ".."
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {useNavigate, useParams} from "react-router-dom"
import {useQuery} from "@apollo/client"
import {
    GetElectionEventQuery,
    GetSupportMaterialsQuery,
    Sequent_Backend_Support_Material,
} from "../gql/graphql"
import ChevronLeftIcon from "@mui/icons-material/ChevronLeft"
import {GET_ELECTION_EVENT} from "../queries/GetElectionEvent"
import {GET_SUPPORT_MATERIALS} from "../queries/GetSupportMaterials"
import {SupportMaterial} from "../components/SupportMaterial/SupportMaterial"
import {
    ISupportMaterial,
    getSupportMaterialsList,
    setSupportMaterial,
} from "../store/supportMaterials/supportMaterialsSlice"
import {
    IElectionEvent,
    selectElectionEventById,
    setElectionEvent,
} from "../store/electionEvents/electionEventsSlice"
import Stepper from "../components/Stepper"

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
    material: Sequent_Backend_Support_Material
}

const ElectionWrapper: React.FC<ElectionWrapperProps> = ({material}) => {
    const {tenantId} = useParams<TenantEventType>()
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

const SupportMaterialsScreen: React.FC = () => {
    const {t, i18n} = useTranslation()
    const navigate = useNavigate()
    const {eventId, tenantId} = useParams<{eventId?: string; tenantId?: string}>()
    const dispatch = useAppDispatch()
    const materials = useAppSelector(getSupportMaterialsList())
    const electionEvent = useAppSelector(selectElectionEventById(eventId))

    const [materialsList, setMaterialsList] = useState<Array<ISupportMaterial> | undefined>([])

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
    })

    useEffect(() => {
        if (!loadingMaterials && !errorMaterials && dataMaterials) {
            for (let material of dataMaterials.sequent_backend_support_material) {
                dispatch(setSupportMaterial(material))
            }
        }
    }, [loadingMaterials, errorMaterials, dataMaterials, dispatch])

    useEffect(() => {
        const materialsList: Array<ISupportMaterial> = []
        for (const material in materials) {
            materialsList.push(materials[material] as ISupportMaterial)
        }
        setMaterialsList(materialsList)
    }, [materials])

    // Election Event
    const {
        data: dataElectionEvent,
        error: errorElectionEvent,
        loading: loadingElectionEvent,
    } = useQuery<GetElectionEventQuery>(GET_ELECTION_EVENT, {
        variables: {
            electionEventId: eventId,
            tenantId,
        },
    })

    useEffect(() => {
        if (!loadingElectionEvent && !errorElectionEvent && dataElectionEvent) {
            for (let material of dataElectionEvent.sequent_backend_election_event) {
                dispatch(setElectionEvent(material))
            }
        }
    }, [loadingElectionEvent, errorElectionEvent, dataElectionEvent, dispatch])

    const [materialsTitles, setMaterialsTitles] = useState<IElectionEvent | undefined>()

    useEffect(() => {
        if (electionEvent) {
            setMaterialsTitles(electionEvent)
        }
    }, [electionEvent])

    const handleNavigateMaterials = () => {
        navigate(`/tenant/${tenantId}/event/${eventId}/election-chooser`)
    }

    return (
        <PageLimit maxWidth="lg">
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
            >
                <Box>
                    <StyledTitle variant="h1">
                        <Box>
                            {materialsTitles &&
                                (translateElection(
                                    materialsTitles,
                                    "materialsTitle",
                                    i18n.language
                                ) ??
                                    "-")}
                        </Box>
                    </StyledTitle>
                    <Typography variant="body1" sx={{color: theme.palette.customGrey.contrastText}}>
                        {stringToHtml(
                            materialsTitles
                                ? translateElection(
                                      materialsTitles,
                                      "materialsSubtitle",
                                      i18n.language
                                  ) ?? "-"
                                : ""
                        )}
                    </Typography>
                </Box>
                <Button startIcon={<ChevronLeftIcon />} onClick={handleNavigateMaterials}>
                    {t("materials.common.back")}
                </Button>
            </Box>
            <ElectionContainer>
                {materialsList?.map((material: ISupportMaterial) => (
                    <ElectionWrapper
                        material={material as Sequent_Backend_Support_Material}
                        key={material.id}
                    />
                ))}
            </ElectionContainer>
        </PageLimit>
    )
}

export default SupportMaterialsScreen
