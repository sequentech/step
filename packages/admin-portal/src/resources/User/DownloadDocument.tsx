// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"
import {FetchDocumentQuery, GetDocumentQuery} from "@/gql/graphql"
import {useQuery, useLazyQuery} from "@apollo/client"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {CircularProgress} from "@mui/material"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {downloadUrl} from "@sequentech/ui-core"
import {GET_DOCUMENT} from "@/queries/GetDocument"

export interface DownloadDocumentProps {
    onDownload: () => void
    fileName: string | null
    documentId: string
    electionEventId?: string
    withProgress?: boolean
    onSuccess?: () => void
}

export const DownloadDocument: React.FC<DownloadDocumentProps> = ({
    onDownload,
    fileName,
    documentId,
    electionEventId,
    withProgress,
    onSuccess,
}) => {
    const {globalSettings} = useContext(SettingsContext)
    const [tenantId] = useTenantStore()
    const [downloadCompleted, setDownloadCompleted] = useState(false)

    const {
        data: hasuraData,
        loading: loadingHasura,
        error: errorHasura,
        stopPolling,
    } = useQuery<GetDocumentQuery>(GET_DOCUMENT, {
        variables: {
            id: documentId,
            tenantId: tenantId,
        },
        skip: !documentId || !tenantId || downloadCompleted,
        pollInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
        onError: (error: any) => {
            console.error(`error checking for document: ${error.message}`)
        },
        onCompleted: (data) => {
            const document = data?.sequent_backend_document?.[0]
            if (document) {
                // Document found, stop polling and trigger the next query
                console.log("Document found, stopping polling.")
                stopPolling()
                getHarvestDocument()
            }
        },
    })

    const [getHarvestDocument, {data: harvestData, loading, error: errorHarvest}] =
        useLazyQuery<FetchDocumentQuery>(FETCH_DOCUMENT, {
            variables: {
                electionEventId,
                documentId,
            },
            fetchPolicy: "network-only", // Don't cache, always get fresh URL
            onCompleted: (data) => {
                console.log("Completed fetching document URL.")
            },
            onError: (error: any) => {
                console.error(`error fetching download URL: ${error.message}`)
                setDownloadCompleted(true)
            },
        })

    let document = hasuraData?.sequent_backend_document?.[0]

    useEffect(() => {
        if (harvestData?.fetchDocument?.url && !downloadCompleted) {
            console.log("Harvest URL received, initiating download.")
            onSuccess?.()

            const name = fileName || document?.name || "file"
            downloadUrl(harvestData.fetchDocument.url, name)
                .then(() => {
                    onDownload()
                })
                .finally(() => {
                    setDownloadCompleted(true)
                })
        }
    }, [harvestData, document, fileName, downloadCompleted, onDownload, onSuccess])

    if (withProgress && (loadingHasura || loading)) {
        return <CircularProgress />
    }

    return null
}
