// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Typography} from "@mui/material"
import React from "react"
import {Edit, ReferenceField, SimpleForm, TextField, TextInput, useRecordContext} from "react-admin"
import {ListAreaContest} from "./ListAreaContest"
import {Sequent_Backend_Area_Contest} from "../../gql/graphql"
import {JsonInput} from "react-admin-json-view"

const AreaContestForm: React.FC = () => {
    return (
        <SimpleForm>
            <Typography variant="h4">Area</Typography>
            <Typography variant="h5">Election Event</Typography>
            <ReferenceField
                label="Election Event"
                reference="sequent_backend_election_event"
                source="election_event_id"
            >
                <TextField source="name" />
            </ReferenceField>
            <Typography variant="h5">Area</Typography>
            <ReferenceField label="Area" reference="sequent_backend_area" source="area_id">
                <TextField source="name" />
            </ReferenceField>
            <Typography variant="h5">Contest</Typography>
            <ReferenceField label="Contest" reference="sequent_backend_contest" source="contest_id">
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

const ListAreaContestWrapper: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Area_Contest>()

    return <ListAreaContest aside={<AreaContestForm />} />
}

export const EditAreaContest: React.FC = () => {
    return (
        <Edit>
            <ListAreaContestWrapper />
        </Edit>
    )
}
