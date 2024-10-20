// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, Button, Typography} from "@mui/material"
import React, {useContext, useEffect, useState} from "react"
import {useTranslation} from "react-i18next"
import {PageLimit, theme} from "@sequentech/ui-essentials"
import {stringToHtml, translate, translateElection} from "@sequentech/ui-core"
import {styled} from "@mui/material/styles"
import {TenantEventType} from ".."
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {useLocation, useNavigate, useParams} from "react-router-dom"
import {GetDocumentQuery, Sequent_Backend_Support_Material} from "../gql/graphql"
import ChevronLeftIcon from "@mui/icons-material/ChevronLeft"
import {SupportMaterial} from "../components/SupportMaterial/SupportMaterial"
import {
    ISupportMaterial,
    getSupportMaterialsList,
} from "../store/supportMaterials/supportMaterialsSlice"
import {IElectionEvent, selectElectionEventById} from "../store/electionEvents/electionEventsSlice"
import Stepper from "../components/Stepper"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {useQuery} from "@apollo/client"
import {GET_DOCUMENT} from "../queries/GetDocument"
import {setDocument} from "../store/documents/documentsSlice"

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
    const location = useLocation()
    const {eventId, tenantId} = useParams<{eventId?: string; tenantId?: string}>()
    const materials = useAppSelector(getSupportMaterialsList())
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const {globalSettings} = useContext(SettingsContext)
    const dispatch = useAppDispatch()

    const [materialsList, setMaterialsList] = useState<Array<ISupportMaterial> | undefined>([])

    const {data: documents} = useQuery<GetDocumentQuery>(GET_DOCUMENT, {
        variables: {
            ids: materialsList?.map((material) => material.document_id ?? "") ?? [],
            electionEventId: eventId,
            tenantId: tenantId || "",
        },
        skip: globalSettings.DISABLE_AUTH,
    })

    useEffect(() => {
        if (globalSettings.DISABLE_AUTH || !documents?.sequent_backend_document) {
            return
        }
        for (let document of documents.sequent_backend_document) {
            dispatch(setDocument(document))
        }
    }, [documents?.sequent_backend_document, globalSettings.DISABLE_AUTH])

    useEffect(() => {
        const materialsList: Array<ISupportMaterial> = []
        for (const material in materials) {
            materialsList.push(materials[material] as ISupportMaterial)
        }
        setMaterialsList(materialsList)
    }, [materials])

    const [materialsTitles, setMaterialsTitles] = useState<IElectionEvent | undefined>()

    useEffect(() => {
        if (electionEvent) {
            setMaterialsTitles(electionEvent)
        }
    }, [electionEvent])

    const handleNavigateMaterials = () => {
        navigate(`/tenant/${tenantId}/event/${eventId}/election-chooser${location.search}`)
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
