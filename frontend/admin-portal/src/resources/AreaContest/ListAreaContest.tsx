// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement} from "react"
import {DatagridConfigurable, List, TextField, ReferenceField} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {useTenantStore} from "../../components/CustomMenu"
import {Typography} from "@mui/material"

const OMIT_FIELDS = ["id"]

export interface ListAreaContestProps {
    electionEventId?: string
    areaId?: string
    aside?: ReactElement
}

export const ListAreaContest: React.FC<ListAreaContestProps> = ({
    aside,
    electionEventId,
    areaId,
}) => {
    const [tenantId] = useTenantStore()

    return (
        <>
            <Typography variant="h5">Area Contests</Typography>
            <List
                actions={<ListActions />}
                sx={{flexGrow: 2}}
                aside={aside}
                filter={{
                    tenant_id: tenantId || undefined,
                    election_event_id: electionEventId,
                    area_id: areaId,
                }}
            >
                <DatagridConfigurable rowClick="edit" omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <ReferenceField
                        label="Election Event"
                        reference="sequent_backend_election_event"
                        source="election_event_id"
                    >
                        <TextField source="name" />
                    </ReferenceField>
                    <ReferenceField label="Area" reference="sequent_backend_area" source="area_id">
                        <TextField source="name" />
                    </ReferenceField>
                    <ReferenceField
                        label="Contest"
                        reference="sequent_backend_contest"
                        source="contest_id"
                    >
                        <TextField source="name" />
                    </ReferenceField>
                </DatagridConfigurable>
            </List>
        </>
    )
}
