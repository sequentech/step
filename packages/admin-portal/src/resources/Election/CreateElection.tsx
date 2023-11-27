// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Typography, styled} from "@mui/material"
import React from "react"
import {
    BooleanInput,
    Create,
    FormDataConsumer,
    NumberInput,
    ReferenceInput,
    SelectInput,
    SimpleForm,
    TextInput,
} from "react-admin"
import {JsonInput} from "react-admin-json-view"

const Hidden = styled(Box)`
    display: none;
`

export const CreateElection: React.FC = () => {
    return (
        <Create>
            <SimpleForm>
                <Typography variant="h4">Create Election</Typography>
                <TextInput source="name" />
                <TextInput source="description" />
                <Hidden>
                    <BooleanInput source="is_consolidated_ballot_encoding" />
                    <BooleanInput source="spoil_ballot_option" />
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
                        source="dates"
                        jsonString={false}
                        reactJsonOptions={{
                            name: null,
                            collapsed: true,
                            enableClipboard: true,
                            displayDataTypes: false,
                        }}
                    />
                    <JsonInput
                        source="status"
                        jsonString={false}
                        reactJsonOptions={{
                            name: null,
                            collapsed: true,
                            enableClipboard: true,
                            displayDataTypes: false,
                        }}
                    />
                    <TextInput source="eml" />
                    <NumberInput source="num_allowed_revotes" />
                </Hidden>
            </SimpleForm>
        </Create>
    )
}
