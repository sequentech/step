// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect} from "react"
import {FetchDocumentQuery} from "@/gql/graphql"
import {useQuery} from "@apollo/client"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"
import {downloadUrl} from "@sequentech/ui-core"
import {SettingsContext} from "@/providers/SettingsContextProvider"

export interface DownloadDocumentProps {
    onDownload: () => void
    fileName: string
    documentId: string
    electionEventId: string
}

export const DownloadDocument: React.FC<DownloadDocumentProps> = ({
    onDownload,
    fileName,
    documentId,
    electionEventId,
}) => {
    const [downloaded, setDownloaded] = React.useState(false)
    const {globalSettings} = useContext(SettingsContext)
    const {loading, error, data} = useQuery<FetchDocumentQuery>(FETCH_DOCUMENT, {
        variables: {
            electionEventId,
            documentId,
        },
        pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
    })

    useEffect(() => {
        console.log(`use effect called filename=${fileName}`)
        if (!error && data?.fetchDocument?.url && !downloaded) {
            setDownloaded(true)
            downloadUrl(data.fetchDocument.url, fileName).then(() => onDownload())
        }
    }, [data, error, loading])

    return <></>
}
