// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    TextInput,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {useTenantStore} from "../../components/CustomMenu"
import {Typography} from "@mui/material"

const OMIT_FIELDS = ["id", "ballot_eml"]

const Filters: Array<ReactElement> = [
    <TextInput label="Name" source="name" key={0} />,
    <TextInput label="ID" source="id" key={1} />,
    <TextInput label="Is Protocol Manager" source="is_protocol_manager" key={2} />,
]

export interface ListTrusteeProps {
    aside?: ReactElement
}

export const ListTrustee: React.FC<ListTrusteeProps> = ({aside}) => {
    const [tenantId] = useTenantStore()

    return (
        <>
            <Typography variant="h5">Trustees</Typography>
            <List
                actions={<ListActions withFilter={true} />}
                sx={{flexGrow: 2}}
                aside={aside}
                filter={{
                    tenant_id: tenantId || undefined,
                }}
                filters={Filters}
            >
                <DatagridConfigurable rowClick="edit" omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="name" />
                    <TextField source="is_protocol_manager" />
                </DatagridConfigurable>
            </List>
        </>
    )
}
