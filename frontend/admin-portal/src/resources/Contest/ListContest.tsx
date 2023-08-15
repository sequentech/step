// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, { PropsWithChildren } from "react"
import { DatagridConfigurable, List, BooleanField, TextField, NumberField } from "react-admin"
import { ListActions } from "../../components/ListActions"

const OMIT_FIELDS = [
    "id", "is_acclaimed", "is_active", "min_votes", "max_votes", "voting_type",
    "counting_algorithm", "is_encrypted"
]

export const ContestList: React.FC<PropsWithChildren> = ({}) => (
    <List actions={<ListActions />} sx={{flexGrow: 2}}>
        <DatagridConfigurable rowClick="edit" omit={OMIT_FIELDS}>
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
        </DatagridConfigurable>
    </List>
)