// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Typography} from "@mui/material"
import React from "react"
import {BooleanInput, Edit, SimpleForm, TextInput, NumberInput, SelectInput} from "react-admin"
import {HorizontalBox} from "../../components/HorizontalBox"
import {ListCandidate} from "./ListCandidate"

const CandidateForm: React.FC = () => {
    return (
        <SimpleForm>
            <Typography variant="h4">Candidate</Typography>
            <Typography variant="body2">Candidate configuration</Typography>
            <TextInput source="name" />
            <TextInput source="description" />
            <TextInput source="type" />
            <BooleanInput source="is_public" />
        </SimpleForm>
    )
}

export const EditCandidate: React.FC = () => {
    return (
        <HorizontalBox>
            <ListCandidate />
            <Edit sx={{flexGrow: 2}}>
                <CandidateForm />
            </Edit>
        </HorizontalBox>
    )
}
