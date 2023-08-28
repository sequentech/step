// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/material"
import React from "react"
import {NumberField, ReferenceField, Show, TextField} from "react-admin"
import { ListDocument } from "./ListDocument"
import { JsonField } from "react-admin-json-view"

export const DocumentProperties: React.FC = () => {
    return (
        <Box sx={{padding: "16px"}}>
            <TextField source="name" fontSize="24px" fontWeight="bold" />
            <TextField source="name" />
            <TextField source="media_type" />
            <NumberField source="size" />
            <ReferenceField
                source="election_event_id"
                reference="sequent_backend_election_event"
            >
                <TextField source="name" />
            </ReferenceField>
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
