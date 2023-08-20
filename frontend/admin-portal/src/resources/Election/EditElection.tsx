// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Button, Typography} from "@mui/material"
import React from "react"
import {
    BooleanInput,
    Edit,
    NumberInput,
    ReferenceField,
    ReferenceManyField,
    SimpleForm,
    TextField,
    TextInput,
    useRecordContext,
} from "react-admin"
import {HorizontalBox} from "../../components/HorizontalBox"
import {ListElection} from "./ListElection"
import {ChipList} from "../../components/ChipList"
import {JsonInput} from "react-admin-json-view"
import {Link} from "react-router-dom"
import {IconButton} from "@sequentech/ui-essentials"
import {Sequent_Backend_Election} from "../../gql/graphql"
import {faPlusCircle} from "@fortawesome/free-solid-svg-icons"

const ElectionForm: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election>()

    return (
        <Box sx={{flexGrow: 2, flexShrink: 0}}>
            <SimpleForm>
                <Typography variant="h4">Election</Typography>
                <Typography variant="body2">Election configuration</Typography>
                <Typography variant="h5">ID</Typography>
                <TextField source="id" />
                <TextInput source="name" />
                <TextInput source="description" />
                <BooleanInput source="is_consolidated_ballot_encoding" />
                <BooleanInput source="spoil_ballot_option" />
                <Typography variant="h5">Election Event</Typography>
                <ReferenceField
                    label="Election Event"
                    reference="sequent_backend_election_event"
                    source="election_event_id"
                >
                    <TextField source="name" />
                </ReferenceField>
                <Typography variant="h5">Contests</Typography>
                <ReferenceManyField
                    label="Contests"
                    reference="sequent_backend_contest"
                    target="election_id"
                >
                    <HorizontalBox>
                        <ChipList
                            source="sequent_backend_contest"
                            filterFields={["election_event_id", "election_id"]}
                        />
                    </HorizontalBox>
                </ReferenceManyField>
                <Link
                    to={{
                        pathname: "/sequent_backend_contest/create",
                    }}
                    state={{
                        record: {
                            election_id: record.id,
                            election_event_id: record.election_event_id,
                            tenant_id: record.tenant_id,
                        },
                    }}
                >
                    <Button>
                        <IconButton icon={faPlusCircle} fontSize="24px" />
                        Add contest
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
                    source="dates"
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
                <TextInput source="eml" />
                <NumberInput source="num_allowed_revotes" />
            </SimpleForm>
        </Box>
    )
}

export const EditElection: React.FC = (props) => (
    <ListElection
        aside={
            <Edit sx={{flexGrow: 2, width: "50%", flexShrink: 0}}>
                <ElectionForm />
            </Edit>
        }
    />
)
