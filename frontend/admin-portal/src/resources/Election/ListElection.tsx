// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren, ReactElement} from "react"
import {BooleanField, DatagridConfigurable, List, ReferenceManyField, TextField} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {ChipList} from "../../components/ChipList"
import {useTenantStore} from "../../components/CustomMenu"
import { Typography } from "@mui/material"

const OMIT_FIELDS = ["id", "is_consolidated_ballot_encoding", "spoil_ballot_option"]

export interface ListElectionProps {
    electionEventId?: string
    aside?: ReactElement
}

export const ListElection: React.FC<ListElectionProps & PropsWithChildren> = ({
    electionEventId, aside
}) => {
    const [tenantId] = useTenantStore()

    return (
        <>
            <Typography variant="h5">Elections</Typography>
            <List
                actions={<ListActions />}
                filter={{tenant_id: tenantId || undefined, election_event_id: electionEventId}}
                aside={aside}
            >
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
                        <ChipList source="sequent_backend_contest" />
                    </ReferenceManyField>
                </DatagridConfigurable>
            </List>
        </>
    )
}
