// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Alert, Box, Button, Typography} from "@mui/material"
import React, {useEffect, useState} from "react"
import {NumberField, ReferenceField, Show, TextField, useRecordContext} from "react-admin"
import {ListDocument} from "./ListDocument"
import {JsonField} from "react-admin-json-view"
import {useApolloClient} from "@apollo/client"
import {FetchDocumentQuery, Sequent_Backend_Document} from "../../gql/graphql"
import {FETCH_DOCUMENT} from "../../queries/FetchDocument"
import {CircularProgress} from "@mui/material"
import {downloadUrl} from "@sequentech/ui-core"

export const DocumentProperties: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Document>()
    const client = useApolloClient()
    const [downloading, setDownloading] = useState(false)
    const [downloadError, setDownloadError] = useState<string>()

    useEffect(() => setDownloadError(undefined), [record?.id])

    const downloadDocument = async () => {
        if (!record || downloading) return
        setDownloading(true)
        setDownloadError(undefined)
        try {
            const {data} = await client.query<FetchDocumentQuery>({
                query: FETCH_DOCUMENT,
                variables: {electionEventId: record.election_event_id, documentId: record.id},
                fetchPolicy: "network-only",
            })
            if (!data?.fetchDocument?.url) throw new Error("Document download unavailable")
            await downloadUrl(data.fetchDocument.url, record.name || "report.pdf")
        } catch (error) {
            setDownloadError(error instanceof Error ? error.message : String(error))
        } finally {
            setDownloading(false)
        }
    }

    if (!record) return null

    return (
        <Box sx={{padding: "16px", display: "flex", flexDirection: "column", gap: "10px"}}>
            <TextField source="name" fontSize="24px" fontWeight="bold" />
            <Typography variant="body1">Media Type</Typography>
            <TextField source="media_type" />
            <Button onClick={downloadDocument} disabled={downloading}>
                Download Document
            </Button>
            {downloading ? <CircularProgress /> : null}
            {downloadError ? <Alert severity="error">{downloadError}</Alert> : null}
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
