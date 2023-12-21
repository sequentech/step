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
    useRecordContext,
} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {ListActions} from "@/components/ListActions"
import {useTranslation} from "react-i18next"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"

const OMIT_FIELDS = ["audit_type", "class", "dbname", "session"]

export interface ElectoralLogListProps {
    aside?: ReactElement
}

export const ElectoralLogList: React.FC<ElectoralLogListProps> = ({aside}) => {
    const [tenantId] = useTenantStore()
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {t} = useTranslation()
    const filters: Array<ReactElement> = [
        <TextInput source="id" key={0} />,
        <TextInput source="statement_kind" key={1} />,
    ]

    return (
        <>
            <List
                resource="electoral_log"
                actions={<ListActions withImport={false} />}
                filters={filters}
                filter={{
                    election_event_id: record?.id || undefined,
                }}
                sort={{
                    field: "id",
                    order: "DESC",
                }}
                aside={aside}
            >
                <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={<></>}>
                    <NumberField source="id" />
                    <FunctionField
                        source="created"
                        render={(record: any) => new Date(record.created / 1000).toUTCString()}
                    />
                    <FunctionField
                        source="statement_timestamp"
                        render={(record: any) =>
                            new Date(record.statement_timestamp / 1000).toUTCString()
                        }
                    />
                    <TextField source="statement_kind" />
                </DatagridConfigurable>
            </List>
        </>
    )
}
