// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Typography} from "@mui/material"
import React from "react"
import {
    SimpleForm,
    TextInput,
    SelectInput,
    ReferenceInput,
    Create,
    FormDataConsumer,
} from "react-admin"

export const CreateArea: React.FC = () => {
    return (
        <Create>
            <SimpleForm>
                <Typography variant="h4">Area</Typography>
                <Typography variant="body2">Area creation</Typography>
                <TextInput source="name" />
                <TextInput source="description" />
                <TextInput source="type" />
                <ReferenceInput source="tenant_id" reference="sequent_backend_tenant">
                    <SelectInput optionText="username" />
                </ReferenceInput>
                <FormDataConsumer>
                    {({formData}) => (
                        <ReferenceInput
                            source="election_event_id"
                            reference="sequent_backend_election_event"
                            filter={{tenant_id: formData.tenant_id}}
                        >
                            <SelectInput optionText="name" />
                        </ReferenceInput>
                    )}
                </FormDataConsumer>
            </SimpleForm>
        </Create>
    )
}
