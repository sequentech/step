// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Typography} from "@mui/material"
import React from "react"
import {
    BooleanInput,
    Edit,
    SimpleForm,
    TextInput,
    NumberInput,
    SelectInput,
    ReferenceManyField,
    useRecordContext,
    ReferenceField,
    TextField,
} from "react-admin"
import {HorizontalBox} from "../../components/HorizontalBox"
import {ListContest} from "./ListContest"
import {ChipList} from "../../components/ChipList"
import {Sequent_Backend_Contest} from "../../gql/graphql"
import {JsonInput} from "react-admin-json-view"

const ContestForm: React.FC = () => {
    return (
        <Box sx={{flexGrow: 2, flexShrink: 0}}>
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
                <Typography variant="h5">Election Event</Typography>
                <ReferenceField
                    label="Election Event"
                    reference="sequent_backend_election_event"
                    source="election_event_id"
                >
                    <TextField source="name" />
                </ReferenceField>
                <Typography variant="h5">Election</Typography>
                <ReferenceField
                    label="Election"
                    reference="sequent_backend_election"
                    source="election_id"
                >
                    <TextField source="name" />
                </ReferenceField>
                <Typography variant="h5">Candidates</Typography>
                <ReferenceManyField
                    label="Candidates"
                    reference="sequent_backend_candidate"
                    target="contest_id"
                >
                    <HorizontalBox>
                        <ChipList source="sequent_backend_candidate" />
                    </HorizontalBox>
                </ReferenceManyField>
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
                    source="tally_configuration"
                    jsonString={false}
                    reactJsonOptions={{
                        name: null,
                        collapsed: true,
                        enableClipboard: true,
                        displayDataTypes: false,
                    }}
                />
                <JsonInput
                    source="conditions"
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

const ListContestWrapper: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Contest>()

    return (
        <ListContest
            electionEventId={record?.election_event_id}
            electionId={record?.election_id}
            aside={<ContestForm />}
        />
    )
}

export const EditContest: React.FC = () => (
    <Edit>
        <ListContestWrapper />
    </Edit>
)
