// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import {FetchDocumentQuery} from "@/gql/graphql"
import {useQuery} from "@apollo/client"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"
import {downloadUrl} from "@sequentech/ui-essentials"
import {SettingsContext} from "@/providers/SettingsContextProvider"

export interface DownloadDocumentProps {
    onDownload: () => void
    fileName: string
    documentId: string
    electionEventId: string
}

let downloading = false

export const DownloadDocument: React.FC<DownloadDocumentProps> = ({
    onDownload,
    fileName,
    documentId,
    electionEventId,
}) => {
    const {globalSettings} = useContext(SettingsContext)
    const {loading, error, data} = useQuery<FetchDocumentQuery>(FETCH_DOCUMENT, {
        variables: {
            electionEventId,
            documentId,
        },
        pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
    })

    if (!loading && !error && data?.fetchDocument?.url && !downloading) {
        downloading = true

        downloadUrl(data.fetchDocument.url, fileName).then(() => onDownload())
    }

    return <></>
}
