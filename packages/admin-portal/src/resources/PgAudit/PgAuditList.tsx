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
} from "react-admin"
import {useTenantStore} from "../../components/CustomMenu"
import {Typography} from "@mui/material"

const OMIT_FIELDS = ["audit_type", "class", "dbname", "session", "user"]

export interface PgAuditListProps {
    aside?: ReactElement
}

export const PgAuditList: React.FC<PgAuditListProps> = ({aside}) => {
    const [tenantId] = useTenantStore()

    return (
        <>
            <Typography variant="h5">PG Audit</Typography>
            <List
                actions={<TopToolbar>
                    <SelectColumnsButton />
                    <ExportButton />
                </TopToolbar>}
                filter={{tenant_id: tenantId || undefined}}
                aside={aside}
            >
                <DatagridConfigurable  omit={OMIT_FIELDS} bulkActionButtons={<></>}>
                    <NumberField source="id" />
                    <TextField source="audit_type" />
                    <TextField source="class" />
                    <TextField source="command" />
                    <TextField source="dbname" />
                    <FunctionField 
                        source="server_timestamp" 
                        render={(record: any) => new Date(record.server_timestamp/1000).toUTCString()}
                    />
                    <TextField source="session_id" />
                    <TextField source="statement" />
                    <TextField source="user" />
                </DatagridConfigurable>
            </List>
        </>
    )
}
