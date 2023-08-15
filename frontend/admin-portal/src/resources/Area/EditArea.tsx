// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Typography} from "@mui/material"
import React from "react"
import {Edit, ReferenceField, SimpleForm, TextField, TextInput} from "react-admin"
import {HorizontalBox} from "../../components/HorizontalBox"
import {ListArea} from "./ListArea"

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
        </SimpleForm>
    )
}

export const EditArea: React.FC = () => {
    return (
        <HorizontalBox>
            <ListArea />
            <Edit sx={{flexGrow: 2}}>
                <AreaForm />
            </Edit>
        </HorizontalBox>
    )
}
