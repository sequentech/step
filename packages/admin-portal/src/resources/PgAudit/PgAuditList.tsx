// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ListActions} from "@/components/ListActions"
import {useTranslation} from "react-i18next"
import {PgAuditTable} from "@/gql/graphql"

const OMIT_FIELDS = ["audit_type", "class", "dbname", "session"]

export interface PgAuditListProps {
    aside?: ReactElement
    auditTable: PgAuditTable
}

export const PgAuditList: React.FC<PgAuditListProps> = ({aside, auditTable}) => {
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()
    const filters: Array<ReactElement> = [
        <TextInput source="id" key={0} />,
        <TextInput source="audit_type" key={1} />,
        <TextInput source="class" key={2} />,
        <TextInput source="command" key={3} />,
        <TextInput source="dbname" key={4} />,
        <TextInput source="session_id" key={5} />,
        <TextInput source="statement" key={6} />,
        <TextInput source="user" key={7} />,
    ]

    return (
        <>
            <List
                resource={auditTable}
                actions={<ListActions withImport={false} />}
                filters={filters}
                storeKey={false}
                aside={aside}
                disableSyncWithLocation
            >
                <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={<></>}>
                    <NumberField source="id" />
                    <TextField source="audit_type" />
                    <TextField source="class" />
                    <TextField source="command" />
                    <TextField source="dbname" />
                    <FunctionField
                        source="server_timestamp"
                        render={(record: any) =>
                            new Date(record.server_timestamp / 1000).toUTCString()
                        }
                    />
                    <TextField source="session_id" />
                    <TextField source="statement" />
                    <TextField source="user" />
                </DatagridConfigurable>
            </List>
        </>
    )
}
