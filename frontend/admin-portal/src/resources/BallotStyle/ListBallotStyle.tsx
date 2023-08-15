// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren} from "react"
import {
    DatagridConfigurable,
    List,
    BooleanField,
    TextField,
    NumberField,
    ReferenceField,
} from "react-admin"
import {ListActions} from "../../components/ListActions"

const OMIT_FIELDS = ["id", "ballot_eml"]

export const ListBallotStyle: React.FC<PropsWithChildren> = () => (
    <List actions={<ListActions />} sx={{flexGrow: 2}}>
        <DatagridConfigurable rowClick="edit" omit={OMIT_FIELDS}>
            <TextField source="id" />
            <TextField source="ballot_eml" />
            <TextField source="status" />
            <ReferenceField label="Area" reference="sequent_backend_area" source="area_id">
                <TextField source="name" />
            </ReferenceField>
            <ReferenceField
                label="Election"
                reference="sequent_backend_election"
                source="election_id"
            >
                <TextField source="name" />
            </ReferenceField>
            <ReferenceField
                label="Election Event"
                reference="sequent_backend_election_event"
                source="election_event_id"
            >
                <TextField source="name" />
            </ReferenceField>
        </DatagridConfigurable>
    </List>
)
