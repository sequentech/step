// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useRef} from "react"
import {FetchDocumentQuery, GetDocumentQuery} from "@/gql/graphql"
import {useQuery} from "@apollo/client"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {CircularProgress} from "@mui/material"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {downloadUrl} from "@sequentech/ui-core"
import {GET_DOCUMENT} from "@/queries/GetDocument"
import {ReportDocumentAccess, ReportPasswordDialog} from "../Reports/ReportPasswordDialog"

export interface DownloadDocumentProps {
    onDownload: () => void
    fileName: string | null
    documentId: string
    electionEventId?: string
    withProgress?: boolean
    onSuccess?: () => void
    showReportPasswordDialog?: boolean
}

export const DownloadDocument: React.FC<DownloadDocumentProps> = ({
    onDownload,
    fileName,
    documentId,
    electionEventId,
    withProgress,
    onSuccess,
    showReportPasswordDialog = false,
}) => {
    const [downloaded, setDownloaded] = React.useState(false)
    const [passwordDocument, setPasswordDocument] = React.useState<{
        scope: string
        access?: ReportDocumentAccess
    }>()
    const downloadStarted = useRef(false)
    const {globalSettings} = useContext(SettingsContext)
    const [tenantId] = useTenantStore()
    const documentScope = JSON.stringify([tenantId, documentId])

    const {data: documents, stopPolling} = useQuery<GetDocumentQuery>(GET_DOCUMENT, {
        variables: {
            id: documentId,
            tenantId: tenantId,
        },
        skip: !documentId || !tenantId || downloaded,
        pollInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
        onError: (error) => {
            console.log(`error downloading doc: ${error.message}`)
        },
        onCompleted: (data) => {
            console.log(`success downloading doc`)
            const document = data?.sequent_backend_document?.[0]
            if (document) {
                // Document found, stop polling and trigger the next query
                console.log("Document found, stopping polling.")
                stopPolling()
                harvestRefetch()
            }
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
            //harvestRefetch()
        },
    })

    let document = documents?.sequent_backend_document?.[0]

    useEffect(() => {
        if (
            !error &&
            data?.fetchDocument?.url &&
            !downloaded &&
            !downloadStarted.current &&
            (!showReportPasswordDialog || document) &&
            (fileName || document)
        ) {
            downloadStarted.current = true
            onSuccess?.()
            console.log("setting downloaded true")

            // Keep the real extension (.epdf for encrypted reports), needed for decryption.
            let name = showReportPasswordDialog
                ? document?.name || fileName || "file"
                : fileName || document?.name || "file"
            console.log("calling downloadUrl")
            downloadUrl(data.fetchDocument.url, name)
                .then(() => {
                    if (
                        showReportPasswordDialog &&
                        (document?.annotations?.access?.password_secret_id ||
                            name.endsWith(".epdf"))
                    ) {
                        // Apollo clears metadata when the completed download skips its query.
                        setPasswordDocument({
                            scope: documentScope,
                            access: document?.annotations?.access,
                        })
                    } else {
                        onDownload()
                    }
                })
                .finally(() => {
                    setDownloaded(true)
                })
        }
    }, [
        data,
        data?.fetchDocument?.url,
        error,
        loading,
        document,
        documentScope,
        fileName,
        downloaded,
        showReportPasswordDialog,
        setDownloaded,
        onDownload,
        downloadUrl,
    ])

    return passwordDocument?.scope === documentScope ? (
        <ReportPasswordDialog
            key={`${tenantId}:${documentId}`}
            documentId={documentId}
            access={passwordDocument.access}
            onClose={() => {
                setPasswordDocument(undefined)
                onDownload()
            }}
        />
    ) : withProgress && !downloaded ? (
        <CircularProgress />
    ) : (
        <></>
    )
}
