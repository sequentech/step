// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Typography} from "@mui/material"
import React from "react"
import {Edit, ReferenceField, SimpleForm, TextField} from "react-admin"
import {ListAreaContest} from "./ListAreaContest"
import {JsonInput} from "react-admin-json-view"

const AreaContestForm: React.FC = () => {
    return (
        <SimpleForm>
            <Typography variant="h4">Area</Typography>
            <Typography variant="h5">Election Event</Typography>
            <Typography variant="h5">ID</Typography>
            <TextField source="id" />
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

export const EditAreaContest: React.FC = () => {
    return (
        <ListAreaContest
            aside={
                <Edit sx={{flexGrow: 2, width: "50%", flexShrink: 0}}>
                    <AreaContestForm />
                </Edit>
            }
        />
    )
}
