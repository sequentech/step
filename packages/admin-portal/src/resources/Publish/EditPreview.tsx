// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useContext, useEffect, useMemo, useState} from "react"
import {
    AutocompleteInput,
    Button,
    Identifier,
    SaveButton,
    SimpleForm,
    Toolbar,
    useNotify,
} from "react-admin"
import {Preview, ContentCopy} from "@mui/icons-material"
import {useTranslation} from "react-i18next"
import {
    GetBallotPublicationChangesOutput,
    GetDocumentByNameQuery,
    PrepareBallotPublicationPreviewMutation,
    Sequent_Backend_Support_Material_Select_Column,
} from "@/gql/graphql"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useLazyQuery, useMutation, useQuery} from "@apollo/client"
import {PREPARE_BALLOT_PUBLICATION_PREVIEW} from "@/queries/PrepareBallotPublicationPreview"
import {GET_AREAS} from "@/queries/GetAreas"
import {TenantContext} from "@/providers/TenantContextProvider"
import {GET_DOCUMENT_BY_NAME} from "@/queries/GetDocumentByName"
import {CircularProgress} from "@mui/material"

enum ActionType {
    Copy,
    Open,
}
interface EditPreviewProps {
    publicationId?: string | Identifier | null
    electionEventId: Identifier | undefined
    close?: () => void
    ballotData: GetBallotPublicationChangesOutput | null
}

export const EditPreview: React.FC<EditPreviewProps> = (props) => {
    const {publicationId: publicationId, close, electionEventId, ballotData} = props
    const {t} = useTranslation()
    const notify = useNotify()
    const {globalSettings} = useContext(SettingsContext)
    const [sourceAreas, setSourceAreas] = useState([])
    const [preparePreview] = useMutation<PrepareBallotPublicationPreviewMutation>(
        PREPARE_BALLOT_PUBLICATION_PREVIEW
    )
    const [isUploading, setIsUploading] = React.useState<boolean>(false)
    const {tenantId} = useContext(TenantContext)
    const [areaId, setAreaId] = useState<string | null>(null)
    const [documentId, setDocumentId] = useState<string | null | undefined>(null)
    const [action, setAction] = useState<ActionType | null>(null)
    const [getDocumentByName] = useLazyQuery<GetDocumentByNameQuery>(GET_DOCUMENT_BY_NAME)
    const {data: areas} = useQuery(GET_AREAS, {
        variables: {
            electionEventId,
        },
    })

    //Show only relevant areas in dropdown
    const areaIds = useMemo(() => {
        const areaIds =
            ballotData?.current?.ballot_styles?.map((style: any) => ({
                id: style.area_id,
            })) || []

        return areaIds
    }, [ballotData])

    useEffect(() => {
        if (areas) {
            const filtered = areas.sequent_backend_area.filter((area: any) =>
                areaIds.some((areaId: any) => areaId.id === area.id)
            )
            setSourceAreas(filtered)
            console.log("filtered", filtered)
        }
    }, [areas, areaIds])

    // If there already is such a document, get it to re-use it
    useEffect(() => {
        const fetchDocumentId = async (documentName: string) => {
            try {
                const {data, error} = await getDocumentByName({
                    variables: {
                        name: documentName,
                        tenantId,
                    },
                    fetchPolicy: "network-only",
                })

                if (error) {
                    console.error("Error fetching document:", error)
                    return false
                }
                const length = data?.sequent_backend_document?.length || 0
                if (length === 0) {
                    return false
                }
                const last = length > 0 ? length - 1 : 0
                return data?.sequent_backend_document[last]?.id
            } catch (err) {
                console.error("Exception in fetchDocumentId:", err)
                return false
            }
        }

        const getDocumentId = async () => {
            const docId = await fetchDocumentId(`${publicationId}.json`)
            setDocumentId(docId)
        }

        if (!documentId) {
            getDocumentId()
        }
    }, [])

    // This useEffect handles file upload
    useEffect(() => {
        const preparePreviewData = async () => {
            try {
                let {data} = await preparePreview({
                    variables: {
                        electionEventId: electionEventId,
                        ballotPublicationId: publicationId,
                    },
                })
                setIsUploading(false) // TODO: Set to false only when the task is completed
                console.log(data)
                if (!data?.prepare_ballot_publication_preview?.document_id) {
                    console.log(data?.prepare_ballot_publication_preview?.error_msg)
                    notify(t("publish.dialog.error_preview"), {type: "error"})
                    return
                }
                notify(t("publish.preview.success"), {type: "success"}) // TODO: Add translations
                return data?.prepare_ballot_publication_preview?.document_id
            } catch (_error) {
                setIsUploading(false)
                notify(t("publish.dialog.error_preview"), {type: "error"}) // TODO: Add translations
                // wip
                return
            }
        }

        const handleDocumentProcess = async () => {
            const docId = await preparePreviewData()
            setDocumentId(docId)
        }
        console.log("isUploading {", isUploading, "}", "areaId {", areaId, "}")
        if (
            isUploading &&
            areaId &&
            undefined !== Sequent_Backend_Support_Material_Select_Column
        ) {
            handleDocumentProcess()
        }
    }, [isUploading, areaId])

    const onPreviewClick = async (res: any) => {
        if (!documentId) {
            setIsUploading(true)
        }
        setAction(ActionType.Open)
    }

    const onCopyPreviewLinkClick = async () => {
        console.log("onCopyPreviewLinkClick", isUploading, documentId)
        if (!documentId) {
            setIsUploading(true)
        }
        setAction(ActionType.Copy)
    }

    // This useEffect handles logic for action (open or copy)
    useEffect(() => {
        const openPreview = (previewUrl: string) => {
            try {
                window.open(previewUrl, "_blank")
                if (close) close()
                notify(t("publish.preview.success"), {type: "success"})
            } catch {
                notify(t("publish.dialog.error_preview"), {type: "error"})
            }
        }

        const copyPreviewLink = async (previewUrl: string) => {
            try {
                await navigator.clipboard.writeText(previewUrl)
                if (close) close()
                notify(t("publish.preview.copy_success"), {type: "success"})
            } catch {
                notify(t("publish.preview.copy_error"), {type: "error"})
            }
        }

        if (documentId && !isUploading) {
            const previewUrl = getPreviewUrl(documentId)
            console.log("previewUrl: ", previewUrl)
            if (previewUrl && action === ActionType.Copy) {
                copyPreviewLink(previewUrl)
            } else if (previewUrl && action === ActionType.Open) {
                openPreview(previewUrl)
            }
        }
    }, [documentId, action, isUploading])

    // Create preview url from data
    const previewUrlTemplate = useMemo(() => {
        return `${globalSettings.VOTING_PORTAL_URL}/preview/${tenantId}`
    }, [globalSettings.VOTING_PORTAL_URL, publicationId])

    const getPreviewUrl = useCallback(
        (documentId: string | undefined | null) => {
            if (!documentId || !areaId || !publicationId) {
                return null
            }
            return `${previewUrlTemplate}/${documentId}/${areaId}/${publicationId}`
        },
        [previewUrlTemplate, areaId, publicationId]
    )

    return (
        <SimpleForm toolbar={false} onSubmit={onPreviewClick}>
            <AutocompleteInput
                source="area_id"
                choices={sourceAreas}
                optionText={(area) => area.name}
                label={t("publish.preview.publicationAreas")}
                fullWidth={true}
                debounce={100}
                onChange={(res) => setAreaId(res)}
            ></AutocompleteInput>
            <Toolbar
                sx={{display: "flex", background: "white", padding: "0 !important", gap: "1rem"}}
            >
                {isUploading ? (
                    <CircularProgress />
                ) : (
                    <>
                        <SaveButton
                            disabled={!areaId}
                            icon={<Preview />}
                            label={t("publish.preview.action")}
                        />
                        <Button
                            disabled={!areaId}
                            startIcon={<ContentCopy />}
                            label={t("publish.preview.copy")}
                            onClick={onCopyPreviewLinkClick}
                        />
                    </>
                )}
            </Toolbar>
        </SimpleForm>
    )
}
