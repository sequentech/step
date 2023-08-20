// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Button, Typography} from "@mui/material"
import React from "react"
import {
    Edit,
    ReferenceField,
    ReferenceManyField,
    SimpleForm,
    TextField,
    TextInput,
    useRecordContext,
} from "react-admin"
import {ListArea} from "./ListArea"
import {JsonInput} from "react-admin-json-view"
import {ChipList} from "../../components/ChipList"
import {Sequent_Backend_Area} from "../../gql/graphql"
import {Link} from "react-router-dom"
import {IconButton} from "@sequentech/ui-essentials"
import {faPlusCircle} from "@fortawesome/free-solid-svg-icons"

const AreaForm: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Area>()

    return (
        <SimpleForm>
            <Typography variant="h4">Area</Typography>
            <Typography variant="body2">Area configuration</Typography>
            <Typography variant="h5">ID</Typography>
            <TextField source="id" />
            <TextInput source="name" />
            <TextInput source="description" />
            <TextInput source="type" />
            <Typography variant="h5">Election Event</Typography>
            <ReferenceField
                label="Election Event"
                reference="sequent_backend_election_event"
                source="election_event_id"
            >
                <TextField source="name" />
            </ReferenceField>
            <ReferenceManyField
                label="Area Contests"
                reference="sequent_backend_area_contest"
                target="area_id"
            >
                <ChipList
                    source="sequent_backend_area_contest"
                    filterFields={["election_event_id", "area_id"]}
                />
            </ReferenceManyField>
            <Link
                to={{
                    pathname: "/sequent_backend_area_contest/create",
                }}
                state={{
                    record: {
                        area_id: record.id,
                        election_event_id: record.election_event_id,
                        tenant_id: record.tenant_id,
                    },
                }}
            >
                <Button>
                    <IconButton icon={faPlusCircle} fontSize="24px" />
                    Add area contest
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
        </SimpleForm>
    )
}

export const EditArea: React.FC = () => {
    return (
        <ListArea
            aside={
                <Edit sx={{flexGrow: 2, width: "50%", flexShrink: 0}}>
                    <AreaForm />
                </Edit>
            }
        />
    )
}
