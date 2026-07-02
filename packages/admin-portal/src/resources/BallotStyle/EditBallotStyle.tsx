// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Typography} from "@mui/material"
import React from "react"
import {
    Edit,
    FormDataConsumer,
    ReferenceField,
    ReferenceInput,
    SelectInput,
    SimpleForm,
    TextField,
    TextInput,
} from "react-admin"
import {ListBallotStyle} from "./ListBallotStyle"
import {JsonInput} from "react-admin-json-view"

const BallotStyleForm: React.FC = () => {
    return (
        <Box sx={{flexGrow: 2, flexShrink: 0}}>
            <SimpleForm>
                <Typography variant="h4">Ballot Style</Typography>
                <Typography variant="body2">Ballot Style configuration</Typography>
                <Typography variant="h5">ID</Typography>
                <TextField source="id" />
                <TextInput source="ballot_eml" />
                <TextInput source="status" />
                <Typography variant="h5">Election</Typography>
                <ReferenceField
                    label="Election"
                    reference="sequent_backend_election"
                    source="election_id"
                >
                    <TextField source="name" />
                </ReferenceField>
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
                            source="area_id"
                            reference="sequent_backend_area"
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
        </Box>
    )
}

export const EditBallotStyle: React.FC = () => {
    return (
        <ListBallotStyle
            aside={
                <Edit sx={{flexGrow: 2, width: "50%", flexShrink: 0}}>
                    <BallotStyleForm />
                </Edit>
            }
        />
    )
}
