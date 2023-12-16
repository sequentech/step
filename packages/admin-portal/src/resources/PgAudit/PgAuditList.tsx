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
} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ListActions} from "@/components/ListActions"
import {useTranslation} from "react-i18next"

const OMIT_FIELDS = ["audit_type", "class", "dbname", "session"]

export interface PgAuditListProps {
    aside?: ReactElement
}

export const PgAuditList: React.FC<PgAuditListProps> = ({aside}) => {
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()
    const filters: Array<ReactElement> = [
        <TextInput label={t("logsScreen.column.id")} source="id" key={0} />,
        <TextInput label={t("logsScreen.column.statement")} source="statement" key={0} />,
    ]

    return (
        <>
            <List
                resource="pgaudit"
                actions={<ListActions withImport={false} />}
                filters={filters}
                aside={aside}
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
