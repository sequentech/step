// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Typography} from "@mui/material"
import React from "react"
import {
    BooleanInput,
    SimpleForm,
    TextInput,
    SelectInput,
    ReferenceInput,
    Create,
    FormDataConsumer,
} from "react-admin"

export const CreateCandidate: React.FC = () => {
    return (
        <Create>
            <SimpleForm>
                <Typography variant="h4">Candidate</Typography>
                <Typography variant="body2">Candidate creation</Typography>
                <TextInput source="name" />
                <TextInput source="description" />
                <TextInput source="type" />
                <BooleanInput source="is_public" />
                <ReferenceInput source="tenant_id" reference="sequent_backend_tenant">
                    <SelectInput optionText="username" />
                </ReferenceInput>
                <FormDataConsumer>
                    {({formData}) => (
                        <>
                            <ReferenceInput
                                source="election_event_id"
                                reference="sequent_backend_election_event"
                                filter={{tenant_id: formData.tenant_id}}
                            >
                                <SelectInput optionText="name" />
                            </ReferenceInput>
                            <ReferenceInput
                                source="election_id"
                                reference="sequent_backend_election"
                                filter={{
                                    tenant_id: formData.tenant_id,
                                    election_event_id: formData.election_event_id,
                                }}
                            >
                                <SelectInput optionText="name" />
                            </ReferenceInput>
                            <ReferenceInput
                                source="contest_id"
                                reference="sequent_backend_contest"
                                filter={{
                                    tenant_id: formData.tenant_id,
                                    election_event_id: formData.election_event_id,
                                    election_id: formData.election_id,
                                }}
                            >
                                <SelectInput optionText="name" />
                            </ReferenceInput>
                        </>
                    )}
                </FormDataConsumer>
            </SimpleForm>
        </Create>
    )
}
