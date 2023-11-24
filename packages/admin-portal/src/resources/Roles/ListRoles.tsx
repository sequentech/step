// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    ExportButton,
    SelectColumnsButton,
    TopToolbar,
} from "react-admin"
import {useTenantStore} from "../../providers/TenantContextProvider"

const OMIT_FIELDS: Array<string> = []

export interface ListRolesProps {
    aside?: ReactElement
    electionEventId?: string
}

export const ListRoles: React.FC<ListRolesProps> = ({aside, electionEventId}) => {
    const [tenantId] = useTenantStore()

    return (
        <>
            <List
                resource="role"
                actions={
                    <TopToolbar>
                        <SelectColumnsButton />
                        <ExportButton />
                    </TopToolbar>
                }
                filter={{tenant_id: tenantId}}
                aside={aside}
            >
                <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={<></>}>
                    <TextField source="id" />
                    <TextField source="name" />
                </DatagridConfigurable>
            </List>
        </>
    )
}
