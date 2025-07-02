// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect} from "react"
import {FetchDocumentQuery, GetDocumentQuery, Sequent_Backend_Document} from "@/gql/graphql"
import {useQuery} from "@apollo/client"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {CircularProgress} from "@mui/material"
import {useGetOne} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {downloadUrl} from "@sequentech/ui-core"
import {GET_DOCUMENT} from "@/queries/GetDocument"

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

    const {data: documents, refetch: hasuraRefetch} = useQuery<GetDocumentQuery>(GET_DOCUMENT, {
        variables: {
            id: documentId,
            tenantId: tenantId,
        },
        skip: !documentId || !tenantId || downloaded,
        pollInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
        onError: (error: any) => {
            console.log(`error downloading doc: ${error.message}`)
        },
        onCompleted: () => {
            console.log(`success downloading doc`)
            harvestRefetch()
        },
    })

    const {
        loading,
        error,
        data,
        refetch: harvestRefetch,
    } = useQuery<FetchDocumentQuery>(FETCH_DOCUMENT, {
        variables: {
            electionEventId,
            documentId,
        },
        skip: !documentId || !tenantId || downloaded,
        pollInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
        onCompleted: () => {
            console.log(`completed fetching document`)
            harvestRefetch()
        },
    })

    let document = documents?.sequent_backend_document?.[0]

    console.log({name: document?.name})

    useEffect(() => {
        if (!error && data?.fetchDocument?.url && !downloaded && (fileName || document)) {
            onSucess && onSucess()
            console.log("setting downloaded true")
            setDownloaded(true)

            let name = fileName || document?.name || "file"
            console.log("calling downloadUrl")
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
