// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement} from "react"
import {DatagridConfigurable, List, TextField, ReferenceField, TextInput} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {Typography} from "@mui/material"
import {generateRowClickHandler} from "../../services/RowClickService"
import {useTenantStore} from "../../providers/TenantContextProvider"

const OMIT_FIELDS = ["id"]

const Filters: Array<ReactElement> = [
    <TextInput source="election_event_id" key={0} />,
    <TextInput source="area_id" key={1} />,
    <TextInput source="contest_id" key={2} />,
]

export interface ListAreaContestProps {
    aside?: ReactElement
}

export const ListAreaContest: React.FC<ListAreaContestProps> = ({aside}) => {
    const [tenantId] = useTenantStore()
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)

    const rowClickHandler = generateRowClickHandler(["election_event_id", "contest_id", "area_id"])

    return (
        <>
            <Typography variant="h5">Area Contests</Typography>
            <List
                actions={<ListActions open={openDrawer} setOpen={setOpenDrawer} />}
                sx={{flexGrow: 2}}
                aside={aside}
                filter={{
                    tenant_id: tenantId || undefined,
                }}
                filters={Filters}
            >
                <DatagridConfigurable rowClick={rowClickHandler} omit={OMIT_FIELDS}>
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
