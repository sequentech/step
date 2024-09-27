// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {PropsWithChildren, ReactElement} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    BooleanField,
    TextInput,
    BooleanInput,
} from "react-admin"
import {ListActions} from "../../components/ListActions"
import {Typography} from "@mui/material"

const OMIT_FIELDS = ["id"]

const Filters: Array<ReactElement> = [
    <TextInput label="Slug" source="slug" key={0} />,
    <TextInput label="ID" source="id" key={2} />,
    <BooleanInput label="Is Active" source="is_active" key={3} />,
]

export interface ListTenantProps {
    aside?: ReactElement
}

export const ListTenant: React.FC<ListTenantProps> = ({aside}) => {
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)
    return (
        <>
            <Typography variant="h5">Tenants</Typography>
            <List
                actions={
                    <ListActions open={openDrawer} setOpen={setOpenDrawer} withFilter={true} />
                }
                sx={{flexGrow: 2}}
                aside={aside}
                filters={Filters}
            >
                <DatagridConfigurable rowClick="edit" omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="slug" />
                    <BooleanField source="is_active" />
                </DatagridConfigurable>
            </List>
        </>
    )
}
