// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Button, Typography} from "@mui/material"
import React, {useState} from "react"
import {NumberField, ReferenceField, Show, TextField, useRecordContext} from "react-admin"
import {ListDocument} from "./ListDocument"
import {JsonField} from "react-admin-json-view"
import {useQuery} from "@apollo/client"
import {FetchDocumentQuery, Sequent_Backend_Document} from "../../gql/graphql"
import {FETCH_DOCUMENT} from "../../queries/FetchDocument"
import {CircularProgress} from "@mui/material"
import {downloadUrl} from "@sequentech/ui-core"

interface PerformDownloadProps {
    onDownload: () => void
}

let downloading = false

const PerformDownload: React.FC<PerformDownloadProps> = ({onDownload}) => {
    const record = useRecordContext<Sequent_Backend_Document>()

    const {loading, error, data} = useQuery<FetchDocumentQuery>(FETCH_DOCUMENT, {
        variables: {
            electionEventId: record.election_event_id,
            documentId: record.id,
        },
    })

    console.log(`is downloading ${downloading}`)
    if (!loading && !error && data?.fetchDocument?.url && !downloading) {
        downloading = true
        console.log("downloading")

        downloadUrl(data.fetchDocument.url, record.name || "report.pdf").then(() => onDownload())
    }

    return <CircularProgress />
}

export const DocumentProperties: React.FC = () => {
    const [performDownload, setPerformDownload] = useState(false)

    const downloadDocument = () => {
        setPerformDownload(true)
    }

    return (
        <Box sx={{padding: "16px", display: "flex", flexDirection: "column", gap: "10px"}}>
            <TextField source="name" fontSize="24px" fontWeight="bold" />
            <Typography variant="body1">Media Type</Typography>
            <TextField source="media_type" />
            <Button onClick={downloadDocument}>Download Document</Button>
            {performDownload ? (
                <PerformDownload
                    onDownload={() => {
                        setPerformDownload(false)
                        downloading = false
                    }}
                />
            ) : null}
            <Typography variant="body1">Size (bytes)</Typography>
            <NumberField source="size" />
            <Typography variant="body1">Election Event</Typography>
            <ReferenceField source="election_event_id" reference="sequent_backend_election_event">
                <TextField source="name" />
            </ReferenceField>
            <Typography variant="body1">Labels</Typography>
            <JsonField
                source="labels"
                jsonString={false}
                reactJsonOptions={{
                    name: null,
                    collapsed: true,
                    enableClipboard: true,
                    displayDataTypes: false,
                }}
            />
            <Typography variant="body1">Annotations</Typography>
            <JsonField
                source="annotations"
                jsonString={false}
                reactJsonOptions={{
                    name: null,
                    collapsed: true,
                    enableClipboard: true,
                    displayDataTypes: false,
                }}
            />
        </Box>
    )
}

export const ShowDocument: React.FC = () => {
    return (
        <ListDocument
            aside={
                <Show sx={{flexGrow: 2, width: "50%", flexShrink: 0}}>
                    <DocumentProperties />
                </Show>
            }
        />
    )
}
