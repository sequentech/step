// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Typography} from "@mui/material"
import React from "react"
import {
    BooleanInput,
    Edit,
    ReferenceManyField,
    SelectInput,
    SimpleForm,
    TextInput,
} from "react-admin"
import {JsonInput} from "react-admin-json-view"
import {ElectionEventList} from "./ElectionEventList"
import {HorizontalBox} from "../../components/HorizontalBox"
import {ChipList} from "../../components/ChipList"

const ElectionEventListForm: React.FC = () => {
    return (
        <SimpleForm>
            <Typography variant="h4">Election Event</Typography>
            <Typography variant="body2">Election event configuration</Typography>
            <TextInput source="name" />
            <TextInput source="description" />
            <SelectInput source="encryption_protocol" choices={[{id: "RSA256", name: "RSA256"}]} />
            <BooleanInput source="is_archived" />
            <BooleanInput source="is_audit" />
            <Typography variant="h5">Elections</Typography>
            <ReferenceManyField
                label="Elections"
                reference="sequent_backend_election"
                target="election_event_id"
            >
                <HorizontalBox>
                    <ChipList source="sequent_backend_election" />
                </HorizontalBox>
            </ReferenceManyField>
            <ReferenceManyField
                label="Areas"
                reference="sequent_backend_area"
                target="election_event_id"
            >
                <ChipList source="sequent_backend_area" />
            </ReferenceManyField>
            <JsonInput
                source="labels"
                jsonString={false}
                reactJsonOptions={{
                    name: null,
                    collapsed: true,
                    enableClipboard: true,
                    displayDataTypes: false,
                }}
            />
            <JsonInput
                source="presentation"
                jsonString={false}
                reactJsonOptions={{
                    name: null,
                    collapsed: true,
                    enableClipboard: true,
                    displayDataTypes: false,
                }}
            />
            <JsonInput
                source="voting_channels"
                jsonString={false}
                reactJsonOptions={{
                    name: null,
                    collapsed: true,
                    enableClipboard: true,
                    displayDataTypes: false,
                }}
            />
            <JsonInput
                source="voting_channels"
                jsonString={false}
                reactJsonOptions={{
                    name: null,
                    collapsed: true,
                    enableClipboard: true,
                    displayDataTypes: false,
                }}
            />
            <JsonInput
                source="dates"
                jsonString={false}
                reactJsonOptions={{
                    name: null,
                    collapsed: true,
                    enableClipboard: true,
                    displayDataTypes: false,
                }}
            />
            <TextInput source="user_boards" />
            <TextInput source="audit_election_event_id" />
        </SimpleForm>
    )
}

export const EditElectionList: React.FC = () => {
    return (
        <ElectionEventList
            aside={
                <Edit sx={{flexGrow: 2}}>
                    <ElectionEventListForm />
                </Edit>
            }
        />
    )
}
