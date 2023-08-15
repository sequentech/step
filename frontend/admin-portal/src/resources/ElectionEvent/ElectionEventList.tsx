// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren} from "react"
import {DatagridConfigurable, List, ReferenceManyField, TextField} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {ElectionChipList} from "../../components/ElectionChipList"

export const ElectionEventList: React.FC<PropsWithChildren> = ({}) => (
    <List actions={<ListActions />} sx={{flexGrow: 2}}>
        <DatagridConfigurable rowClick="edit" omit={["id"]}>
            <TextField source="id" />
            <TextField source="name" />
            <TextField source="description" />
            <ReferenceManyField
                label="Elections"
                reference="sequent_backend_election"
                target="election_event_id"
            >
                <ElectionChipList />
            </ReferenceManyField>
        </DatagridConfigurable>
    </List>
)
