// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Typography} from "@mui/material"
import React from "react"
import {
    BooleanInput,
    Edit,
    FormDataConsumer,
    ReferenceField,
    ReferenceInput,
    SelectInput,
    SimpleForm,
    TextField,
    TextInput,
} from "react-admin"
import {HorizontalBox} from "../../components/HorizontalBox"
import {ListBallotStyle} from "./ListBallotStyle"

const BallotStyleForm: React.FC = () => {
    return (
        <SimpleForm>
            <Typography variant="h4">Ballot Style</Typography>
            <Typography variant="body2">Ballot Style configuration</Typography>
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
        </SimpleForm>
    )
}

export const EditBallotStyle: React.FC = () => {
    return (
        <HorizontalBox>
            <ListBallotStyle />
            <Edit sx={{flexGrow: 2}}>
                <BallotStyleForm />
            </Edit>
        </HorizontalBox>
    )
}
