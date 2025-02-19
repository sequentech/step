// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect} from "react"
import {FetchDocumentQuery, Sequent_Backend_Document} from "@/gql/graphql"
import {useQuery} from "@apollo/client"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {CircularProgress} from "@mui/material"
import {useGetOne} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {downloadUrl} from "@sequentech/ui-core"

export interface DownloadDocumentProps {
    onDownload: () => void
    fileName: string | null
    documentId: string
    electionEventId?: string
    withProgress?: boolean
    onSucess?: () => void
}

export const DownloadDocument: React.FC<DownloadDocumentProps> = ({
    onDownload,
    fileName,
    documentId,
    electionEventId,
    withProgress,
    onSucess,
}) => {
    const [downloaded, setDownloaded] = React.useState(false)
    const {globalSettings} = useContext(SettingsContext)
    const [tenantId] = useTenantStore()

    const {data: document} = useGetOne<Sequent_Backend_Document>(
        "sequent_backend_document",
        {
            id: documentId,
            meta: {tenant_id: tenantId},
        },
        {
            enabled: !!documentId,
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            onError: (error: any) => {
                console.log(`error downloading doc: ${error.message}`)
            },
            onSuccess: () => {
                console.log(`success downloading doc`)
            },
        }
    )

    const {loading, error, data} = useQuery<FetchDocumentQuery>(FETCH_DOCUMENT, {
        variables: {
            electionEventId,
            documentId,
        },
        pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
    })

    console.log({name: document?.name})

    useEffect(() => {
        if (!error && data?.fetchDocument?.url && !downloaded && (fileName || document)) {
            onSucess && onSucess()
            setDownloaded(true)

            let name = fileName || document?.name || "file"
            downloadUrl(data.fetchDocument.url, name).then(() => onDownload())
        }
    }, [
        data,
        data?.fetchDocument?.url,
        error,
        loading,
        document,
        fileName,
        downloaded,
        setDownloaded,
        onDownload,
        downloadUrl,
    ])

    return withProgress ? <CircularProgress /> : <></>
}
