// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    FunctionField,
    TextInput,
    NumberField,
    ExportButton,
    SelectColumnsButton,
    TopToolbar,
    BooleanField,
} from "react-admin"
import {useTenantStore} from "../../components/CustomMenu"
import {Typography} from "@mui/material"

const OMIT_FIELDS: Array<string> = []

export interface ListUsersProps {
    aside?: ReactElement
}

export const ListUsers: React.FC<ListUsersProps> = ({aside}) => {
    const [tenantId] = useTenantStore()

    return (
        <>
            <Typography variant="h5">Users</Typography>
            <List
                resource="user"
                actions={
                    <TopToolbar>
                        <SelectColumnsButton />
                        <ExportButton />
                    </TopToolbar>
                }
                filter={{tenant_id: tenantId || "90505c8a-23a9-4cdf-a26b-4e19f6a097d5"}}
                aside={aside}
            >
                <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={<></>}>
                    <NumberField source="id" />
                    <TextField source="email" />
                    <BooleanField source="email_verified" />
                    <BooleanField source="enabled" />
                    <TextField source="first_name" />
                    <TextField source="last_name" />
                    <TextField source="username" />
                </DatagridConfigurable>
            </List>
        </>
    )
}
