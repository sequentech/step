// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    BooleanField,
    BooleanInput,
    DatagridConfigurable,
    List,
    ReferenceManyField,
    TextField,
    TextInput,
} from "react-admin"

import React, {ReactElement} from "react"

import {ChipList} from "../../components/ChipList"
import {CreateElectionList} from "./CreateElectionEvent"
import ElectionHeader from "../../components/ElectionHeader"
import {ListActions} from "../../components/ListActions"
import {useTenantStore} from "../../components/CustomMenu"
import {Link} from "react-router-dom"
import {Button} from "@mui/material"
import {IconButton} from "@sequentech/ui-essentials"
import {faPlusCircle} from "@fortawesome/free-solid-svg-icons"

const OMIT_FIELDS = ["id", "sequent_backend_area", "is_archived", "is_audit", "public_key"]

const Filters: Array<ReactElement> = [
    <TextInput label="Name" source="name" key={0} />,
    <TextInput label="Description" source="description" key={1} />,
    <TextInput label="ID" source="id" key={2} />,
    <BooleanInput label="Is Archived" source="is_archived" key={3} />,
    <BooleanInput label="Is Audit" source="is_audit" key={4} />,
    <TextInput label="Public Key" source="public_key" key={5} />,
]

export interface ElectionEventListProps {
    aside?: ReactElement
}

export const ElectionEventList: React.FC<ElectionEventListProps> = ({aside}) => {
    const [tenantId] = useTenantStore()

    const actions = <ListActions withFilter={true} Component={<CreateElectionList />} />

    const empty = (
        <Link style={{padding: "16px"}} to="/sequent_backend_election_event/create">
            <Button>
                <IconButton icon={faPlusCircle} fontSize="24px" />
                Create new election event
            </Button>
        </Link>
    )

    return (
        <>
            <ElectionHeader title="Election Events" subtitle="Election Events Subtitle" />
            <List
                actions={actions}
                filter={{tenant_id: tenantId || undefined}}
                filters={Filters}
                aside={aside}
                empty={empty}
            >
                <DatagridConfigurable rowClick="edit" omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="name" />
                    <TextField source="description" />
                    <BooleanField source="is_archived" />
                    <BooleanField source="is_audit" />
                    <TextField source="public_key" />
                    <ReferenceManyField
                        label="Elections"
                        reference="sequent_backend_election"
                        target="election_event_id"
                    >
                        <ChipList
                            source="sequent_backend_election"
                            filterFields={["election_event_id"]}
                        />
                    </ReferenceManyField>
                    <ReferenceManyField
                        label="Areas"
                        reference="sequent_backend_area"
                        target="election_event_id"
                    >
                        <ChipList
                            source="sequent_backend_area"
                            filterFields={["election_event_id"]}
                        />
                    </ReferenceManyField>
                </DatagridConfigurable>
            </List>
        </>
    )
}
