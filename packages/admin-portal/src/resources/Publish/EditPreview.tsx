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
    useGetList,
    useGetOne,
    useNotify,
} from "react-admin"
import {Preview, ContentCopy} from "@mui/icons-material"
import {useTranslation} from "react-i18next"
import {
    GetBallotPublicationChangesOutput,
    GetDocumentByNameQuery,
    GetUploadUrlMutation,
    PrepareBallotPublicationPreviewMutation,
    Sequent_Backend_Document,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Support_Material,
} from "@/gql/graphql"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useLazyQuery, useMutation, useQuery} from "@apollo/client"
import {PREPARE_BALLOT_PUBLICATION_PREVIEW} from "@/queries/PrepareBallotPublicationPreview"
import {GET_AREAS} from "@/queries/GetAreas"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"
import {TenantContext} from "@/providers/TenantContextProvider"
import {GET_DOCUMENT_BY_NAME} from "@/queries/GetDocumentByName"
import {ElectionEventStatus} from "./EPublishStatus"
import {CircularProgress} from "@mui/material"

enum ActionType {
    Copy,
    Open,
}
interface EditPreviewProps {
    id?: string | Identifier | null
    electionEventId: Identifier | undefined
    close?: () => void
    ballotData: GetBallotPublicationChangesOutput | null
}

export const EditPreview: React.FC<EditPreviewProps> = (props) => {
    const {id, close, electionEventId, ballotData} = props
    const {t} = useTranslation()
    const notify = useNotify()
    const {globalSettings} = useContext(SettingsContext)
    const [sourceAreas, setSourceAreas] = useState([])
    const [getUploadUrl] = useMutation<GetUploadUrlMutation>(GET_UPLOAD_URL)
    const [preparePreview] = useMutation<PrepareBallotPublicationPreviewMutation>(PREPARE_BALLOT_PUBLICATION_PREVIEW)

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

    const {data: electionEvent} = useGetOne<Sequent_Backend_Election_Event>(
        "sequent_backend_election_event",
        {
            id: electionEventId,
        },
        {
            refetchIntervalInBackground: true,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const {data: elections} = useGetList<Sequent_Backend_Election>(
        "sequent_backend_election",
        {
            pagination: {page: 1, perPage: 9999},
            sort: {field: "created_at", order: "DESC"},
            filter: {
                election_event_id: electionEventId,
                tenant_id: tenantId,
            },
        },
        {
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const {data: supportMaterials} = useGetList<Sequent_Backend_Support_Material>(
        "sequent_backend_support_material",
        {
            pagination: {page: 1, perPage: 9999},
            sort: {field: "created_at", order: "DESC"},
            filter: {
                is_hidden: false,
                election_event_id: electionEventId,
                tenant_id: tenantId,
            },
        },
        {
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const {data: documents} = useGetList<Sequent_Backend_Document>(
        "sequent_backend_document",
        {
            pagination: {page: 1, perPage: 9999},
            sort: {field: "created_at", order: "DESC"},
            filter: {
                election_event_id: electionEventId,
                tenant_id: tenantId,
            },
        },
        {
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

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

                return data?.sequent_backend_document[0]?.id
            } catch (err) {
                console.error("Exception in fetchDocumentId:", err)
                return false
            }
        }

        const getDocumentId = async () => {
            const docId = await fetchDocumentId(`${id}.json`)
            setDocumentId(docId)
        }

        if (!documentId) {
            getDocumentId()
        }
    }, [])

    // This useEffect handles file upload
    useEffect(() => {
        const uploadFile = async (url: string, file: File) => {
            await fetch(url, {
                method: "PUT",
                headers: {
                    "Content-Type": file.type,
                },
                body: file,
            })
            setIsUploading(false)
        }

        const uploadFileToS3 = async (theFile: File) => {
            try {
                let {data} = await getUploadUrl({
                    variables: {
                        name: theFile.name,
                        media_type: theFile.type,
                        size: theFile.size,
                        is_public: true,
                    },
                })

                if (!data?.get_upload_url?.url) {
                    notify(t("electionEventScreen.import.fileUploadError"), {type: "error"})
                    return
                }

                await uploadFile(data.get_upload_url.url, theFile)
                return data.get_upload_url.document_id
            } catch (_error) {
                setIsUploading(false)
                notify(t("electionEventScreen.import.fileUploadError"), {type: "error"})
            }
        }

        const updateElectionStatus = (elections: Array<Sequent_Backend_Election> | undefined) => {
            return elections?.map((election) => {
                if (election?.status) {
                    return {
                        ...election,
                        status: {
                            ...election.status,
                            voting_status: ElectionEventStatus.Open,
                        },
                    }
                }
                return election
            })
        }

        const prepareFileData = () => {
            const openElections = updateElectionStatus(elections)

            if (electionEvent?.status) {
                electionEvent.status.voting_status = ElectionEventStatus.Open
            }

            return {
                ballot_styles: ballotData?.current?.ballot_styles,
                election_event: electionEvent,
                elections: openElections,
                support_materials: supportMaterials,
                documents: documents,
            }
        }

        const preparePreviewData = async () => {
            try {
                let {data} = await preparePreview({
                    variables: {
                        electionEventId: electionEventId,
                        ballotPublicationId: id,
                    },
                })

                if (!data?.prepare_ballot_publication_preview?.document_id) {
                    notify(t("publish.preview.success"), {type: "success"}) // TODO: Add translations
                    return
                }
                // wip

            } catch (_error) {
                notify(t("publish.dialog.error_preview"), {type: "error"}) // TODO: Add translations
                // wip
            }
        }

        const startUpload = async () => {
            const fileData = prepareFileData()
            const dataStr = JSON.stringify(fileData, null, 2)
            const file = new File([dataStr], `${id}.json`, {type: "application/json"})
            console.log(file)
            // new endpoint test
            await preparePreviewData()
            const docId = await uploadFileToS3(file)
            setDocumentId(docId)
        }

        const handleDocumentProcess = async () => {
            await startUpload()
        }

        if (
            isUploading &&
            electionEvent &&
            elections &&
            areaId &&
            undefined !== supportMaterials &&
            undefined !== documents
        ) {
            handleDocumentProcess()
        }
    }, [isUploading, electionEvent, elections, areaId, supportMaterials, documents])

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

        if (documentId) {
            const previewUrl = getPreviewUrl(documentId)
            if (previewUrl && action === ActionType.Copy) {
                copyPreviewLink(previewUrl)
            } else if (previewUrl && action === ActionType.Open) {
                openPreview(previewUrl)
            }
        }
    }, [documentId, action])

    // Create preview url from data
    const previewUrlTemplate = useMemo(() => {
        return `${globalSettings.VOTING_PORTAL_URL}/preview/${tenantId}`
    }, [globalSettings.VOTING_PORTAL_URL, id])

    const getPreviewUrl = useCallback(
        (documentId: string | undefined | null) => {
            if (!documentId || !areaId || !id) {
                return null
            }
            return `${previewUrlTemplate}/${documentId}/${areaId}/${id}`
        },
        [previewUrlTemplate, areaId, id]
    )

    const onPreviewClick = async (res: any) => {
        setIsUploading(true)
        setAction(ActionType.Open)
    }

    const onCopyPreviewLinkClick = async () => {
        if (!documentId) {
            setIsUploading(true)
        }
        setAction(ActionType.Copy)
    }

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
