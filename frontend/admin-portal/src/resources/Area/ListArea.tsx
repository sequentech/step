// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren} from "react"
import {DatagridConfigurable, List, TextField, ReferenceField} from "react-admin"
import {ListActions} from "../../components/ListActions"

const OMIT_FIELDS = ["id", "ballot_eml"]

export const ListArea: React.FC<PropsWithChildren> = () => (
    <List actions={<ListActions />} sx={{flexGrow: 2}}>
        <DatagridConfigurable rowClick="edit" omit={OMIT_FIELDS}>
            <TextField source="id" />
            <TextField source="name" />
            <TextField source="description" />
            <TextField source="type" />
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
