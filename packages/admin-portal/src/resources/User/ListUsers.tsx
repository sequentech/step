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
    electionEventId?: string
}

export const ListUsers: React.FC<ListUsersProps> = ({aside, electionEventId}) => {
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
                filter={{tenant_id: tenantId, election_event_id: electionEventId}}
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
