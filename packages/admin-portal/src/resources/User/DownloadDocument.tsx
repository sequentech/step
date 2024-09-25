// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect} from "react"
import {FetchDocumentQuery, Sequent_Backend_Document} from "@/gql/graphql"
import {useQuery} from "@apollo/client"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"
import {downloadUrl} from "@sequentech/ui-core"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {CircularProgress} from "@mui/material"
import {useGetOne} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"

export interface DownloadDocumentProps {
    onDownload: () => void
    fileName: string | null
    documentId: string
    electionEventId: string
    withProgress?: boolean
}

export const DownloadDocument: React.FC<DownloadDocumentProps> = ({
    onDownload,
    fileName,
    documentId,
    electionEventId,
    withProgress,
}) => {
    const [downloaded, setDownloaded] = React.useState(false)
    const {globalSettings} = useContext(SettingsContext)
    const [tenantId] = useTenantStore()

    const {data: document} = useGetOne<Sequent_Backend_Document>("sequent_backend_document", {
        id: documentId,
        meta: {tenant_id: tenantId},
    })

    const {loading, error, data} = useQuery<FetchDocumentQuery>(FETCH_DOCUMENT, {
        variables: {
            electionEventId,
            documentId,
        },
        pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
    })

    useEffect(() => {
        if (!error && data?.fetchDocument?.url && !downloaded && (fileName || document)) {
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
        downloadUrl,
        onDownload,
    ])

    return withProgress ? <CircularProgress /> : <></>
}
