// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement} from "react"
import {
    BooleanField,
    BooleanInput,
    DatagridConfigurable,
    List,
    ReferenceManyField,
    TextField,
    TextInput,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {useTenantStore} from "../../components/CustomMenu"
import {ChipList} from "../../components/ChipList"
import {Typography} from "@mui/material"

const OMIT_FIELDS = ["id", "sequent_backend_area", "is_archived", "is_audit"]

const Filters: Array<ReactElement> = [
    <TextInput label="Name" source="name" key={0} />,
    <TextInput label="Description" source="description" key={1} />,
    <TextInput label="ID" source="id" key={2} />,
    <BooleanInput label="Is Archived" source="is_archived" key={3} />,
    <BooleanInput label="Is Audit" source="is_audit" key={4} />,
]

export interface ElectionEventListProps {
    aside?: ReactElement
}

export const ElectionEventList: React.FC<ElectionEventListProps> = ({aside}) => {
    const [tenantId] = useTenantStore()

    return (
        <>
            <Typography variant="h5">Election Events</Typography>
            <List
                actions={<ListActions withFilter={true} />}
                filter={{tenant_id: tenantId || undefined}}
                filters={Filters}
                aside={aside}
            >
                <DatagridConfigurable rowClick="edit" omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="name" />
                    <TextField source="description" />
                    <BooleanField source="is_archived" />
                    <BooleanField source="is_audit" />
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
