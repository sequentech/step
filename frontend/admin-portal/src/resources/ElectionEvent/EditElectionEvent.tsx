// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Typography} from "@mui/material"
import React from "react"
import {
    BooleanInput,
    Edit,
    ReferenceManyField,
    SelectInput,
    SimpleForm,
    TextInput,
} from "react-admin"
import {ElectionEventList} from "./ElectionEventList"
import {ElectionChipList} from "../../components/ElectionChipList"
import {HorizontalBox} from "../../components/HorizontalBox"

const ElectionEventListForm: React.FC = () => {
    return (
        <SimpleForm>
            <Typography variant="h4">Election Event</Typography>
            <Typography variant="body2">Election event configuration</Typography>
            <TextInput source="name" />
            <TextInput source="description" />
            <SelectInput source="encryption_protocol" choices={[{id: "RSA256", name: "RSA256"}]} />
            <BooleanInput source="is_archived" />
            <BooleanInput source="is_audit" />
            <ReferenceManyField
                label="Elections"
                reference="sequent_backend_election"
                target="election_event_id"
            >
                <HorizontalBox>
                    <ElectionChipList />
                </HorizontalBox>
            </ReferenceManyField>
        </SimpleForm>
    )
}

export const EditElectionList: React.FC = () => {
    return (
        <HorizontalBox>
            <ElectionEventList />
            <Edit sx={{flexGrow: 2}}>
                <ElectionEventListForm />
            </Edit>
        </HorizontalBox>
    )
}
