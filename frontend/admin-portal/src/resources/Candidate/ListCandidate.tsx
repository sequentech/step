// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, { ReactElement } from "react"
import {DatagridConfigurable, List, BooleanField, TextField, NumberField} from "react-admin"
import {ListActions} from "../../components/ListActions"
import { useTenantStore } from "../../components/CustomMenu"
import { Typography } from "@mui/material"

const OMIT_FIELDS = ["id", "type", "is_public"]

export interface ListCandidateProps {
    electionEventId?: string
    contestId?: string
    aside?: ReactElement
}

export const ListCandidate: React.FC<ListCandidateProps> = ({electionEventId, contestId, aside}) => {
    const [tenantId] = useTenantStore()

    return <>
        <Typography variant="h5">Candidates</Typography>
        <List
            actions={<ListActions />}
            sx={{flexGrow: 2}}
            filter={{
                tenant_id: tenantId || undefined,
                election_event_id: electionEventId,
                contest_id: contestId,
            }}
            aside={aside}
        >
            <DatagridConfigurable rowClick="edit" omit={OMIT_FIELDS}>
                <TextField source="id" />
                <TextField source="name" />
                <TextField source="description" />
                <TextField source="type" />
                <BooleanField source="is_public" />
            </DatagridConfigurable>
        </List>
    </>
}
