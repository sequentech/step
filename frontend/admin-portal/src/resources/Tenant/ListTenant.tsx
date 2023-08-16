// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren, ReactElement} from "react"
import {DatagridConfigurable, List, TextField, ReferenceField, BooleanField} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {useTenantStore} from "../../components/CustomMenu"
import {Typography} from "@mui/material"

const OMIT_FIELDS = ["id"]

export interface ListTenantProps {
    aside?: ReactElement
}

export const ListTenant: React.FC<ListTenantProps> = ({aside}) => {
    return (
        <>
            <Typography variant="h5">Customers</Typography>
            <List actions={<ListActions />} sx={{flexGrow: 2}} aside={aside}>
                <DatagridConfigurable rowClick="edit" omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="username" />
                    <BooleanField source="is_active" />
                </DatagridConfigurable>
            </List>
        </>
    )
}
