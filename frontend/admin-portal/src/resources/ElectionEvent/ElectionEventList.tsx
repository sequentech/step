// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement} from "react"
import {DatagridConfigurable, List, ReferenceManyField, TextField} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {useTenantStore} from "../../components/CustomMenu"
import {ChipList} from "../../components/ChipList"
import { Box, Typography } from "@mui/material"

const OMIT_FIELDS = ["id", "sequent_backend_area"]

export interface ElectionEventListProps {
    aside?: ReactElement
}

export const ElectionEventList: React.FC<ElectionEventListProps> = ({aside}) => {
    const [tenantId] = useTenantStore()

    return (
        <>
            <Typography variant="h5">Election Events</Typography>
            <List
                actions={<ListActions />}
                filter={{tenant_id: tenantId || undefined}}
                aside={aside}
            >
                <DatagridConfigurable rowClick="edit" omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="name" />
                    <TextField source="description" />
                    <ReferenceManyField
                        label="Elections"
                        reference="sequent_backend_election"
                        target="election_event_id"
                    >
                        <ChipList source="sequent_backend_election" />
                    </ReferenceManyField>
                    <ReferenceManyField
                        label="Areas"
                        reference="sequent_backend_area"
                        target="election_event_id"
                    >
                        <ChipList source="sequent_backend_area" />
                    </ReferenceManyField>
                </DatagridConfigurable>
            </List>
        </>
    )
}
