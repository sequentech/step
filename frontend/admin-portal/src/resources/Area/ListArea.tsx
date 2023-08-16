// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    ReferenceField,
    ReferenceManyField,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {useTenantStore} from "../../components/CustomMenu"
import {Typography} from "@mui/material"
import {ChipList} from "../../components/ChipList"

const OMIT_FIELDS = ["id", "ballot_eml"]

export interface ListAreaProps {
    electionEventId?: string
    aside?: ReactElement
}

export const ListArea: React.FC<ListAreaProps> = ({aside, electionEventId}) => {
    const [tenantId] = useTenantStore()

    return (
        <>
            <Typography variant="h5">Areas</Typography>
            <List
                actions={<ListActions />}
                sx={{flexGrow: 2}}
                aside={aside}
                filter={{
                    tenant_id: tenantId || undefined,
                    election_event_id: electionEventId,
                }}
            >
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
                    <ReferenceManyField
                        label="Area Contests"
                        reference="sequent_backend_area_contest"
                        target="area_id"
                    >
                        <ChipList source="sequent_backend_area_contest" />
                    </ReferenceManyField>
                </DatagridConfigurable>
            </List>
        </>
    )
}
