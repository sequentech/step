// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren} from "react"
import {DatagridConfigurable, List, ReferenceManyField, TextField} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {ElectionChipList} from "../../components/ElectionChipList"
import { AreaChipList } from "../../components/AreaChipList"

const OMIT_FIELDS = ["id", "sequent_backend_area"]

export const ElectionEventList: React.FC<PropsWithChildren> = () => (
    <List actions={<ListActions />} sx={{flexGrow: 2}}>
        <DatagridConfigurable rowClick="edit" omit={OMIT_FIELDS}>
            <TextField source="id" />
            <TextField source="name" />
            <TextField source="description" />
            <ReferenceManyField
                label="Elections"
                reference="sequent_backend_election"
                source="sequent_backend_election"
                target="election_event_id"
            >
                <ElectionChipList />
            </ReferenceManyField>
            <ReferenceManyField
                label="Areas"
                reference="sequent_backend_area"
                source="sequent_backend_area"
                target="election_event_id"
            >
                <AreaChipList />
            </ReferenceManyField>
        </DatagridConfigurable>
    </List>
)
