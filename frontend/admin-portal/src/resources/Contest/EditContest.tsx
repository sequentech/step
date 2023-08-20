// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Button, Typography} from "@mui/material"
import React from "react"
import {
    BooleanInput,
    Edit,
    SimpleForm,
    TextInput,
    NumberInput,
    SelectInput,
    ReferenceManyField,
    ReferenceField,
    TextField,
    useRecordContext,
} from "react-admin"
import {HorizontalBox} from "../../components/HorizontalBox"
import {ListContest} from "./ListContest"
import {ChipList} from "../../components/ChipList"
import {JsonInput} from "react-admin-json-view"
import {Sequent_Backend_Contest} from "../../gql/graphql"
import {Link} from "react-router-dom"
import {IconButton} from "@sequentech/ui-essentials"
import {faPlusCircle} from "@fortawesome/free-solid-svg-icons"

const ContestForm: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Contest>()

    return (
        <Box sx={{flexGrow: 2, flexShrink: 0}}>
            <SimpleForm>
                <Typography variant="h4">Contest</Typography>
                <Typography variant="body2">Contest configuration</Typography>
                <Typography variant="h5">ID</Typography>
                <TextField source="id" />
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
                        <ChipList
                            source="sequent_backend_candidate"
                            filterFields={["election_event_id", "contest_id"]}
                        />
                    </HorizontalBox>
                </ReferenceManyField>
                <Link
                    to={{
                        pathname: "/sequent_backend_candidate/create",
                    }}
                    state={{
                        record: {
                            contest_id: record.id,
                            election_event_id: record.election_event_id,
                            tenant_id: record.tenant_id,
                        },
                    }}
                >
                    <Button>
                        <IconButton icon={faPlusCircle} fontSize="24px" />
                        Add candidate
                    </Button>
                </Link>
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

export const EditContest: React.FC = () => (
    <ListContest
        aside={
            <Edit sx={{flexGrow: 2, width: "50%", flexShrink: 0}}>
                <ContestForm />
            </Edit>
        }
    />
)
