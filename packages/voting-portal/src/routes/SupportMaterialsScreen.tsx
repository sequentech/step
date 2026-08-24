// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Alert, Box, Button, Checkbox, FormControlLabel, Typography} from "@mui/material"
import React, {useContext, useEffect, useMemo, useState} from "react"
import {useTranslation} from "react-i18next"
import {PageLimit, theme} from "@sequentech/ui-essentials"
import {
    stringToHtml,
    translate,
    translateFromPresentation,
    ESupportMaterialsPolicy,
    getEffectiveSupportMaterialsPolicy,
} from "@sequentech/ui-core"
import {styled} from "@mui/material/styles"
import {TenantEventType} from ".."
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {selectFirstBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import {useLocation, useNavigate, useParams} from "react-router-dom"
import {
    AcknowledgeSupportMaterialsMutation,
    GetDocumentQuery,
    GetSupportMaterialsAcknowledgmentQuery,
    Sequent_Backend_Support_Material,
} from "../gql/graphql"
import ChevronLeftIcon from "@mui/icons-material/ChevronLeft"
import {SupportMaterial} from "../components/SupportMaterial/SupportMaterial"
import {
    ISupportMaterial,
    getSupportMaterialsList,
} from "../store/supportMaterials/supportMaterialsSlice"
import {IElectionEvent, selectElectionEventById} from "../store/electionEvents/electionEventsSlice"
import Stepper from "../components/Stepper"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {useMutation, useQuery} from "@apollo/client/react"
import {GET_DOCUMENT} from "../queries/GetDocument"
import {setDocument} from "../store/documents/documentsSlice"
import {ACKNOWLEDGE_SUPPORT_MATERIALS} from "../queries/AcknowledgeSupportMaterials"
import {GET_SUPPORT_MATERIALS_ACKNOWLEDGMENT} from "../queries/GetSupportMaterialsAcknowledgment"

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
    onViewed?: () => void
}

const ElectionWrapper: React.FC<ElectionWrapperProps> = ({material, onViewed}) => {
    const {tenantId} = useParams<TenantEventType>()
    const {i18n} = useTranslation()

    return (
        <SupportMaterial
            title={translate(material.data, "title", i18n.language) || ""}
            subtitle={translate(material.data, "subtitle", i18n.language) || ""}
            kind={material.kind || ""}
            tenantId={tenantId || ""}
            documentId={material.document_id || ""}
            onViewed={onViewed}
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
    const ballotStyle = useAppSelector(selectFirstBallotStyle)
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
    const defaultLanguageCode = materialsTitles?.presentation?.language_conf?.default_language_code

    useEffect(() => {
        if (electionEvent) {
            setMaterialsTitles(electionEvent)
        }
    }, [electionEvent])

    // Sourced from the published ballot style snapshot, not the live election
    // event, so a policy change only takes effect after the next publication.
    const materialsPolicy = getEffectiveSupportMaterialsPolicy(
        ballotStyle?.ballot_eml.election_event_presentation?.materials
    )
    const isMandatory = materialsPolicy === ESupportMaterialsPolicy.MANDATORY_FOR_VOTING

    const [viewedIds, setViewedIds] = useState<Set<string>>(new Set())
    const [acknowledgeChecked, setAcknowledgeChecked] = useState(false)
    const [acknowledgeError, setAcknowledgeError] = useState<string | undefined>()
    const [acknowledging, setAcknowledging] = useState(false)
    const [acknowledgeSupportMaterials] = useMutation<AcknowledgeSupportMaterialsMutation>(
        ACKNOWLEDGE_SUPPORT_MATERIALS,
        {
            update: (cache, {data}) => {
                if (!eventId || !data?.acknowledge_support_materials) {
                    return
                }
                cache.writeQuery<GetSupportMaterialsAcknowledgmentQuery>({
                    query: GET_SUPPORT_MATERIALS_ACKNOWLEDGMENT,
                    variables: {electionEventId: eventId},
                    data: {
                        get_support_materials_acknowledgment: {
                            __typename: "GetSupportMaterialsAcknowledgmentOutput",
                            document_ids: data.acknowledge_support_materials.document_ids,
                        },
                    },
                })
            },
        }
    )

    const allMaterialsViewed = useMemo(
        () => (materialsList ?? []).every((material) => viewedIds.has(material.id)),
        [materialsList, viewedIds]
    )

    const handleNavigateMaterials = () => {
        navigate(`/tenant/${tenantId}/event/${eventId}/election-chooser${location.search}`)
    }

    const handleContinue = async () => {
        if (!eventId) {
            return
        }
        setAcknowledging(true)
        setAcknowledgeError(undefined)
        try {
            const documentIds = (materialsList ?? [])
                .map((material) => material.document_id)
                .filter((documentId): documentId is string => Boolean(documentId))
            const result = await acknowledgeSupportMaterials({
                variables: {electionEventId: eventId, documentIds},
            })
            if (result.error) {
                setAcknowledgeError(t("materials.mandatory.error"))
                return
            }
            handleNavigateMaterials()
        } catch (error) {
            console.log(error)
            setAcknowledgeError(t("materials.mandatory.error"))
        } finally {
            setAcknowledging(false)
        }
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
                                (translateFromPresentation(
                                    materialsTitles,
                                    "materialsTitle",
                                    i18n.language,
                                    {defaultLanguageCode}
                                ) ??
                                    "-")}
                        </Box>
                    </StyledTitle>
                    <Typography variant="body1" sx={{color: theme.palette.customGrey.contrastText}}>
                        {stringToHtml(
                            materialsTitles
                                ? (translateFromPresentation(
                                      materialsTitles,
                                      "materialsSubtitle",
                                      i18n.language,
                                      {defaultLanguageCode}
                                  ) ?? "-")
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
                        onViewed={() => setViewedIds((prev) => new Set(prev).add(material.id))}
                    />
                ))}
            </ElectionContainer>
            {isMandatory ? (
                <Box sx={{marginTop: "20px"}}>
                    {acknowledgeError ? (
                        <Alert severity="error" sx={{marginBottom: "16px"}}>
                            {acknowledgeError}
                        </Alert>
                    ) : null}
                    <FormControlLabel
                        control={
                            <Checkbox
                                checked={acknowledgeChecked}
                                disabled={!allMaterialsViewed}
                                onChange={(event) => setAcknowledgeChecked(event.target.checked)}
                                inputProps={{
                                    "aria-label": t("materials.mandatory.checkboxLabel"),
                                }}
                            />
                        }
                        label={t("materials.mandatory.checkboxLabel")}
                    />
                    <Box sx={{marginTop: "16px"}}>
                        <Button
                            className="materials-continue-button"
                            disabled={!acknowledgeChecked || acknowledging}
                            onClick={handleContinue}
                        >
                            {t("materials.mandatory.continueButton")}
                        </Button>
                    </Box>
                </Box>
            ) : null}
        </PageLimit>
    )
}

export default SupportMaterialsScreen
