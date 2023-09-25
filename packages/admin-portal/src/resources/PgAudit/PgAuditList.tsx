// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
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

const OMIT_FIELDS = ["id"]

const Filters: Array<ReactElement> = [
    <TextInput label="Command" source="comand" key={0} />,
    <TextInput label="Statement" source="statement" key={1} />,
    <TextInput label="ID" source="id" key={2} />,
]

export interface PgAuditListProps {
    aside?: ReactElement
}

export const PgAuditList: React.FC<PgAuditListProps> = ({aside}) => {
    const [tenantId] = useTenantStore()

    return (
        <>
            <Typography variant="h5">PG Audit</Typography>
            <List
                actions={<ListActions withFilter={true} />}
                filter={{tenant_id: tenantId || undefined}}
                filters={Filters}
                aside={aside}
            >
                <DatagridConfigurable omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <TextField source="statement" />
                    <TextField source="command" />
                </DatagridConfigurable>
            </List>
        </>
    )
}
