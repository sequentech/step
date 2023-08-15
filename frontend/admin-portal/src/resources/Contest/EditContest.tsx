// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Typography} from "@mui/material"
import React from "react"
import {
    BooleanInput,
    Edit,
    SimpleForm,
    TextInput,
    NumberInput,
    SelectInput,
    ReferenceManyField,
} from "react-admin"
import {HorizontalBox} from "../../components/HorizontalBox"
import {CandidateChipList} from "../../components/CandidateChipList"
import {ListContest} from "./ListContest"

const ContestForm: React.FC = () => {
    return (
        <SimpleForm>
            <Typography variant="h4">Contest</Typography>
            <Typography variant="body2">Contest configuration</Typography>
            <TextInput source="name" />
            <TextInput source="description" />
            <BooleanInput source="is_acclaimed" />
            <BooleanInput source="is_active" />
            <NumberInput source="min_votes" />
            <NumberInput source="max_votes" />
            <SelectInput
                source="voting_type"
                choices={[{id: "first-past-the-post", name: "First Past The Post"}]}
            />
            <SelectInput
                source="counting_algorithm"
                choices={[{id: "plurality-at-large", name: "Plurality At Large"}]}
            />
            <BooleanInput source="is_encrypted" />
            <Typography variant="h5">Candidates</Typography>
            <ReferenceManyField
                label="Candidates"
                reference="sequent_backend_candidate"
                target="contest_id"
            >
                <HorizontalBox>
                    <CandidateChipList />
                </HorizontalBox>
            </ReferenceManyField>
        </SimpleForm>
    )
}

export const EditContest: React.FC = () => {
    return (
        <HorizontalBox>
            <ListContest />
            <Edit sx={{flexGrow: 2}}>
                <ContestForm />
            </Edit>
        </HorizontalBox>
    )
}
