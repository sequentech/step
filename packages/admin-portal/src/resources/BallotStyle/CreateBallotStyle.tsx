// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
import {JsonInput} from "react-admin-json-view"

export const CreateBallotStyle: React.FC = () => {
    return (
        <Create>
            <SimpleForm>
                <Typography variant="h4">Ballot Style</Typography>
                <Typography variant="body2">Ballot Style creation</Typography>
                <TextInput source="ballot_eml" />
                <TextInput source="status" />
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
                                source="area_id"
                                reference="sequent_backend_area"
                                filter={{
                                    tenant_id: formData.tenant_id,
                                    election_event_id: formData.election_event_id,
                                }}
                            >
                                <SelectInput optionText="name" />
                            </ReferenceInput>
                        </>
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
                    source="status"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
            </SimpleForm>
        </Create>
    )
}
