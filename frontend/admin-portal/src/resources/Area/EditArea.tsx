// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Typography} from "@mui/material"
import React from "react"
import {Edit, ReferenceField, SimpleForm, TextField, TextInput, useRecordContext} from "react-admin"
import {ListArea} from "./ListArea"
import {Sequent_Backend_Area} from "../../gql/graphql"
import {JsonInput} from "react-admin-json-view"

const AreaForm: React.FC = () => {
    return (
        <SimpleForm>
            <Typography variant="h4">Area</Typography>
            <Typography variant="body2">Area configuration</Typography>
            <TextInput source="name" />
            <TextInput source="description" />
            <TextInput source="type" />
            <Typography variant="h5">Election Event</Typography>
            <ReferenceField
                label="Election Event"
                reference="sequent_backend_election_event"
                source="election_event_id"
            >
                <TextField source="name" />
            </ReferenceField>
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
                source="annotations"
                jsonString={false}
                reactJsonOptions={{
                    name: null,
                    collapsed: true,
                    enableClipboard: true,
                    displayDataTypes: false,
                }}
            />
        </SimpleForm>
    )
}

const ListAreaWrapper: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Area>()

    return <ListArea electionEventId={record?.election_event_id} aside={<AreaForm />} />
}

export const EditArea: React.FC = () => {
    return (
        <Edit>
            <ListAreaWrapper />
        </Edit>
    )
}
