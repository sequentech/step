// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement} from "react"
import {
    DatagridConfigurable,
    List,
    BooleanField,
    TextField,
    NumberField,
    ReferenceManyField,
    TextInput,
    BooleanInput,
    NumberInput,
    ReferenceField,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {ChipList} from "../../components/ChipList"
import {Typography} from "@mui/material"
import {generateRowClickHandler} from "../../services/RowClickService"
import {useTenantStore} from "../../providers/TenantContextProvider"

const OMIT_FIELDS = [
    "id",
    "is_acclaimed",
    "is_active",
    "min_votes",
    "max_votes",
    "voting_type",
    "counting_algorithm",
    "is_encrypted",
]

const Filters: Array<ReactElement> = [
    <TextInput label="Name" source="name" key={0} />,
    <TextInput label="Description" source="description" key={1} />,
    <TextInput label="ID" source="id" key={2} />,
    <BooleanInput label="Is Acclaimed" source="is_acclaimed" key={3} />,
    <BooleanInput label="Is Active" source="is_active" key={4} />,
    <NumberInput label="Min Votes" source="min_votes" key={5} />,
    <NumberInput label="Max Votes" source="max_votes" key={6} />,
    <TextInput label="Voting Type" source="voting_type" key={7} />,
    <TextInput label="Counting Algorithm" source="counting_algorithm" key={8} />,
    <BooleanInput label="Is Encrypted" source="is_encrypted" key={9} />,
    <TextInput source="election_event_id" key={10} />,
    <TextInput source="election_id" key={11} />,
]

export interface ListContestProps {
    aside?: ReactElement
}

export const ListContest: React.FC<ListContestProps> = ({aside}) => {
    const [tenantId] = useTenantStore()
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)

    const rowClickHandler = generateRowClickHandler(["election_event_id", "election_id"])

    return (
        <>
            <Typography variant="h5">Contests</Typography>
            <List
                actions={
                    <ListActions open={openDrawer} setOpen={setOpenDrawer} withFilter={true} />
                }
                sx={{flexGrow: 2}}
                filter={{
                    tenant_id: tenantId || undefined,
                }}
                filters={Filters}
                aside={aside}
            >
                <DatagridConfigurable rowClick={rowClickHandler} omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="name" />
                    <TextField source="description" />
                    <BooleanField source="is_acclaimed" />
                    <BooleanField source="is_active" />
                    <NumberField source="min_votes" />
                    <NumberField source="max_votes" />
                    <TextField source="voting_type" />
                    <TextField source="counting_algorithm" />
                    <BooleanField source="is_encrypted" />
                    <ReferenceManyField
                        label="Candidates"
                        reference="sequent_backend_candidate"
                        target="contest_id"
                    >
                        <ChipList
                            source="sequent_backend_candidate"
                            filterFields={["election_event_id", "contest_id"]}
                        />
                    </ReferenceManyField>
                    <ReferenceField
                        source="election_event_id"
                        reference="sequent_backend_election_event"
                    >
                        <TextField source="name" />
                    </ReferenceField>
                    <ReferenceField source="election_id" reference="sequent_backend_election">
                        <TextField source="name" />
                    </ReferenceField>
                </DatagridConfigurable>
            </List>
        </>
    )
}
