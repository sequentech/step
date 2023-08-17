// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Typography} from "@mui/material"
import React from "react"
import {
    Edit,
    ReferenceField,
    ReferenceManyField,
    SimpleForm,
    TextField,
    TextInput,
} from "react-admin"
import {ListArea} from "./ListArea"
import {JsonInput} from "react-admin-json-view"
import {ChipList} from "../../components/ChipList"

const AreaForm: React.FC = () => {
    return (
        <SimpleForm>
            <Typography variant="h4">Area</Typography>
            <Typography variant="body2">Area configuration</Typography>
            <Typography variant="h5">ID</Typography>
            <TextField source="id" />
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
            <ReferenceManyField
                label="Area Contests"
                reference="sequent_backend_area_contest"
                target="area_id"
            >
                <ChipList
                    source="sequent_backend_area_contest"
                    filterFields={["election_event_id", "area_id"]}
                />
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

export const EditArea: React.FC = () => {
    return (
        <Edit>
            <ListArea aside={<AreaForm />} />
        </Edit>
    )
}
