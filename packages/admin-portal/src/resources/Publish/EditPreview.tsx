// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useMemo, useState} from "react"
import {
    AutocompleteInput,
    Identifier,
    SaveButton,
    SimpleForm,
    useGetList,
    useGetOne,
    useNotify,
} from "react-admin"
import {Preview} from "@mui/icons-material"
import {useTranslation} from "react-i18next"
import {
    GetBallotPublicationChangesOutput,
    GetDocumentByNameQuery,
    GetUploadUrlMutation,
    Sequent_Backend_Document,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Support_Material,
} from "@/gql/graphql"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useLazyQuery, useMutation, useQuery} from "@apollo/client"
import {GET_AREAS} from "@/queries/GetAreas"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"
import {TenantContext} from "@/providers/TenantContextProvider"
import {GET_DOCUMENT_BY_NAME} from "@/queries/GetDocumentByName"
import { ElectionEventStatus } from "./EPublishStatus"

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
    const [isUploading, setIsUploading] = React.useState<boolean>(false)
    const {tenantId} = useContext(TenantContext)
    const [areaId, setAreaId] = useState<string | null>(null)
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

            const documentId = data?.sequent_backend_document[0]?.id
            console.log({documentId})
            if (documentId) {
                const openSuccess = openPreview(documentId)
                if (openSuccess) {
                    return true
                }
            }

            return false
        } catch (err) {
            console.error("Exception in fetchDocumentId:", err)
            return false
        }
    }

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
            notify(t("electionEventScreen.import.fileUploadSuccess"), {type: "success"})
            return data.get_upload_url.document_id
        } catch (_error) {
            setIsUploading(false)
            notify(t("electionEventScreen.import.fileUploadError"), {type: "error"})
        }
    }

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

    useEffect(() => {
        const startUpload = async () => {
            const openElections = elections?.filter(election => election.status?.voting_status === ElectionEventStatus.Open);
            const fileData = {
                ballot_styles: ballotData?.current?.ballot_styles,
                election_event: electionEvent,
                elections: openElections,
                support_materials: supportMaterials,
                documents: documents,
            }
            const dataStr = JSON.stringify(fileData, null, 2)
            const file = new File([dataStr], `${id}.json`, {type: "application/json"})
            const documentId = await uploadFileToS3(file)
            openPreview(documentId)

            if (close) {
                close()
            }
        }

        const handleDocumentProcess = async () => {
            const documentOpened = await fetchDocumentId(`${id}.json`)
            console.log({documentOpened})
            if (!documentOpened) {
                await startUpload()
            }
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

    const openPreview = (documentId: string | undefined | null) => {
        if (documentId) {
            const previewUrl: string = `${previewUrlTemplate}/${documentId}/${areaId}/${id}`
            window.open(previewUrl, "_blank")
            notify(t("publish.preview.success"), {type: "success"})
            return true
        } else {
            notify(t("publish.dialog.error_preview"), {type: "error"})
            return false
        }
    }

    const onPreviewClick = async (res: any) => {
        setAreaId(res.area_id)
        setIsUploading(true)
    }

    const previewUrlTemplate = useMemo(() => {
        return `${globalSettings.VOTING_PORTAL_URL}/preview/${tenantId}`
    }, [globalSettings.VOTING_PORTAL_URL, id])

    return (
        <SimpleForm
            onSubmit={onPreviewClick}
            toolbar={
                <SaveButton
                    icon={<Preview />}
                    label={t("publish.preview.action")}
                    sx={{marginInline: "1rem"}}
                />
            }
        >
            <AutocompleteInput
                source="area_id"
                choices={sourceAreas}
                optionText={(area) => area.name}
                label={t("publish.preview.publicationAreas")}
                fullWidth={true}
                debounce={100}
            ></AutocompleteInput>
        </SimpleForm>
    )
}
