// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import { Typography } from "@mui/material"
import React from "react"
import { BooleanInput, SimpleForm, TextInput, NumberInput, SelectInput, ReferenceInput, Create } from "react-admin"

export const CreateContest: React.FC = () => {
    return (
        <Create>
            <SimpleForm>
                <Typography variant="h4">Contest</Typography>
                <Typography variant="body2">Contest configuration</Typography>
                <TextInput source="name" />
                <TextInput source="description" />
                <BooleanInput source="is_acclaimed" />
                <BooleanInput source="is_active" />
                <NumberInput source="min_votes" />
                <NumberInput source="max_votes" />
                <SelectInput source="voting_type" choices={[{id: "first-past-the-post", name: "First Past The Post"}]} />
                <SelectInput source="counting_algorithm" choices={[{id: "plurality-at-large", name: "Plurality At Large"}]} />
                <BooleanInput source="is_encrypted" />
                <ReferenceInput source="tenant_id" reference="sequent_backend_tenant">
                    <SelectInput optionText="username" />
                </ReferenceInput>
                <ReferenceInput source="election_event_id" reference="sequent_backend_election_event">
                    <SelectInput optionText="name" />
                </ReferenceInput>
                <ReferenceInput source="election_id" reference="sequent_backend_election">
                    <SelectInput optionText="name" />
                </ReferenceInput>
            </SimpleForm>
        </Create>
    )
}