// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Typography} from "@mui/material"
import React from "react"
import {
    BooleanInput,
    Edit,
    SimpleForm,
    TextInput,
    SelectInput,
    ReferenceField,
    TextField,
    FormDataConsumer,
    ReferenceInput,
} from "react-admin"
import {ListCandidate} from "./ListCandidate"
import {JsonInput} from "react-admin-json-view"

const CandidateForm: React.FC = () => {
    return (
        <Box sx={{flexGrow: 2, flexShrink: 0}}>
            <SimpleForm>
                <Typography variant="h4">Candidate</Typography>
                <Typography variant="body2">Candidate configuration</Typography>
                <Typography variant="h5">ID</Typography>
                <TextField source="id" />
                <TextInput source="name" />
                <TextInput source="description" />
                <TextInput source="type" />
                <BooleanInput source="is_public" />
                <Typography variant="h5">Election Event</Typography>
                <ReferenceField
                    label="Election Event"
                    reference="sequent_backend_election_event"
                    source="election_event_id"
                >
                    <TextField source="name" />
                </ReferenceField>
                <FormDataConsumer>
                    {({formData}) => (
                        <ReferenceInput
                            source="contest_id"
                            reference="sequent_backend_contest"
                            filter={{
                                tenant_id: formData.tenant_id,
                                election_event_id: formData.election_event_id,
                            }}
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
            </SimpleForm>
        </Box>
    )
}

export const EditCandidate: React.FC = () => {
    return (
        <ListCandidate
            aside={
                <Edit sx={{flexGrow: 2, width: "50%", flexShrink: 0}}>
                    <CandidateForm />
                </Edit>
            }
        />
    )
}
