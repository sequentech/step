// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Typography} from "@mui/material"
import React from "react"
import {
    BooleanInput,
    Edit,
    ReferenceManyField,
    SimpleForm,
    TextInput,
    useRecordContext,
    useResourceContext,
} from "react-admin"
import {HorizontalBox} from "../../components/HorizontalBox"
import {ListElection} from "./ListElection"
import {ChipList} from "../../components/ChipList"
import {Sequent_Backend_Election} from "../../gql/graphql"

const ElectionForm: React.FC = () => {
    return (
        <Box sx={{flexGrow: 2, flexShrink: 0}}>
            <SimpleForm>
                <Typography variant="h4">Election</Typography>
                <Typography variant="body2">Election configuration</Typography>
                <TextInput source="name" />
                <TextInput source="description" />
                <BooleanInput source="is_consolidated_ballot_encoding" />
                <BooleanInput source="spoil_ballot_option" />
                <Typography variant="h5">Contests</Typography>
                <ReferenceManyField
                    label="Contests"
                    reference="sequent_backend_contest"
                    target="election_id"
                >
                    <HorizontalBox>
                        <ChipList source="sequent_backend_contest" />
                    </HorizontalBox>
                </ReferenceManyField>
            </SimpleForm>
        </Box>
    )
}

const ListElectionWrapper: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election>()

    return <ListElection electionEventId={record?.election_event_id} />
}

export const EditElection: React.FC = (props) => (
    <Edit>
        <HorizontalBox>
            <ListElectionWrapper />
            <ElectionForm />
        </HorizontalBox>
    </Edit>
)
