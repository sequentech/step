// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import { Typography } from "@mui/material"
import React from "react"
import { BooleanInput, Edit, ReferenceManyField, SimpleForm, TextInput } from "react-admin"
import { HorizontalBox } from "../../components/HorizontalBox"
import { ContestChipList } from "../../components/ContestChipList"
import { ElectionList } from "./ListElection"

const ElectionForm: React.FC = () => {
    return (
        <SimpleForm>
            <Typography variant="h4">Election</Typography>
            <Typography variant="body2">Election configuration</Typography>
            <TextInput source="name" />
            <TextInput source="description" />
            <BooleanInput source="is_consolidated_ballot_encoding" />
            <BooleanInput source="spoil_ballot_option" />
            <ReferenceManyField
                label="Contests"
                reference="sequent_backend_contest"
                target="election_id"
            >
                <HorizontalBox>
                    <ContestChipList />
                </HorizontalBox>
            </ReferenceManyField>
        </SimpleForm>
    )
}

export const EditElection: React.FC = () => {
    return (
        <HorizontalBox>
            <ElectionList />
            <Edit sx={{flexGrow: 2, flexShrink: 0}}>
                <ElectionForm />
            </Edit>
        </HorizontalBox>
    )
}