// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement} from "react"
import {DatagridConfigurable, List, TextField, ReferenceField, TextInput} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {Typography} from "@mui/material"
import {useTenantStore} from "../../providers/TenantContextProvider"

const OMIT_FIELDS = ["id", "ballot_eml"]

const Filters: Array<ReactElement> = [
    <TextInput label="Ballot EML" source="ballot_eml" key={0} />,
    <TextInput label="ID" source="id" key={1} />,
    <TextInput label="Status" source="status" key={2} />,
    <TextInput source="area_id" key={3} />,
    <TextInput source="election_event_id" key={4} />,
    <TextInput source="election_id" key={5} />,
]

export const ListBallotStyle: React.FC = () => {
    const [tenantId] = useTenantStore()
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)

    return (
        <>
            <Typography variant="h5">Ballot Styles</Typography>
            <List
                actions={
                    <ListActions open={openDrawer} setOpen={setOpenDrawer} withFilter={true} />
                }
                sx={{flexGrow: 2}}
                filter={{
                    tenant_id: tenantId || undefined,
                }}
                filters={Filters}
            >
                <DatagridConfigurable omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="ballot_eml" />
                    <TextField source="status" />
                    <ReferenceField label="Area" reference="sequent_backend_area" source="area_id">
                        <TextField source="name" />
                    </ReferenceField>
                    <ReferenceField
                        label="Election"
                        reference="sequent_backend_election"
                        source="election_id"
                    >
                        <TextField source="name" />
                    </ReferenceField>
                    <ReferenceField
                        label="Election Event"
                        reference="sequent_backend_election_event"
                        source="election_event_id"
                    >
                        <TextField source="name" />
                    </ReferenceField>
                    <ReferenceField source="area_id" reference="sequent_backend_area">
                        <TextField source="name" />
                    </ReferenceField>
                </DatagridConfigurable>
            </List>
        </>
    )
}
