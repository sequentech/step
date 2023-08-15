// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren} from "react"
import {DatagridConfigurable, List, BooleanField, TextField, NumberField} from "react-admin"
import {ListActions} from "../../components/ListActions"

const OMIT_FIELDS = ["id", "type", "is_public"]

export const ListCandidate: React.FC<PropsWithChildren> = () => (
    <List actions={<ListActions />} sx={{flexGrow: 2}}>
        <DatagridConfigurable rowClick="edit" omit={OMIT_FIELDS}>
            <TextField source="id" />
            <TextField source="name" />
            <TextField source="description" />
            <TextField source="type" />
            <BooleanField source="is_public" />
        </DatagridConfigurable>
    </List>
)
