// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren} from "react"
import {BooleanField, DatagridConfigurable, List, ReferenceManyField, TextField} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {ContestChipList} from "../../components/ContestChipList"

const OMIT_FIELDS = ["id", "is_consolidated_ballot_encoding", "spoil_ballot_option"]

export const ListElection: React.FC<PropsWithChildren> = ({}) => (
    <List actions={<ListActions />} sx={{flexGrow: 2}}>
        <DatagridConfigurable rowClick="edit" omit={OMIT_FIELDS}>
            <TextField source="id" />
            <TextField source="name" />
            <TextField source="description" />
            <BooleanField source="is_consolidated_ballot_encoding" />
            <BooleanField source="spoil_ballot_option" />
            <ReferenceManyField
                label="Contests"
                reference="sequent_backend_contest"
                target="election_id"
            >
                <ContestChipList />
            </ReferenceManyField>
        </DatagridConfigurable>
    </List>
)
