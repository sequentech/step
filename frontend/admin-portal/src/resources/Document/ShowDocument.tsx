// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Typography} from "@mui/material"
import React from "react"
import {NumberField, ReferenceField, Show, TextField} from "react-admin"
import {ListDocument} from "./ListDocument"
import {JsonField} from "react-admin-json-view"

export const DocumentProperties: React.FC = () => {
    return (
        <Box sx={{padding: "16px", display: "flex", flexDirection: "column", gap: "10px"}}>
            <TextField source="name" fontSize="24px" fontWeight="bold" />
            <Typography variant="body1">Media Type</Typography>
            <TextField source="media_type" />
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
