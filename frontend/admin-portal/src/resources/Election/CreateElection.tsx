// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import { Typography } from "@mui/material"
import React from "react"
import { BooleanInput, Create, Edit, ReferenceInput, ReferenceManyField, SelectInput, SimpleForm, TextInput } from "react-admin"
import { HorizontalBox } from "../../components/HorizontalBox"
import { ContestChipList } from "../../components/ContestChipList"
import { ElectionList } from "./ListElection"

export const CreateElection: React.FC = () => {
    return (
        <Create>
            <SimpleForm>
                <Typography variant="h4">Create Election</Typography>
                <TextInput source="name" />
                <TextInput source="description" />
                <BooleanInput source="is_consolidated_ballot_encoding" />
                <BooleanInput source="spoil_ballot_option" />
                <ReferenceInput source="tenant_id" reference="sequent_backend_tenant">
                    <SelectInput optionText="username" />
                </ReferenceInput>
                <ReferenceInput source="election_event_id" reference="sequent_backend_election_event">
                    <SelectInput optionText="name" />
                </ReferenceInput>
            </SimpleForm>
        </Create>
    )
}