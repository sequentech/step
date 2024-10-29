// SPDX-FileCopyrightText: 2024 Félix Robles <dev@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useEffect, useState} from "react"
import {
    List,
    TextInput,
    DateField,
    FunctionField,
    TextField,
    DatagridConfigurable,
    Identifier,
} from "react-admin"
import {useTranslation} from "react-i18next"
import {Visibility} from "@mui/icons-material"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {ListActions} from "@/components/ListActions"
import {StatusChip} from "@/components/StatusChip"
import {ResetFilters} from "@/components/ResetFilters"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {useMutation} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
// import {EXPORT_TASKS_EXECUTION} from "@/queries/ExportApprovalExecution"
// import {ExportApprovalExecutionMutation, Sequent_Backend_Election_Event} from "@/gql/graphql"

export interface ListApprovalsProps {
    electionEventId: string
    electionId?: string
    onViewApproval: (id: Identifier) => void
    electionEventRecord: Sequent_Backend_Election_Event
}
export const ListApprovals: React.FC<ListApprovalsProps> = ({
    electionEventId,
    electionId,
    onViewApproval,
    electionEventRecord,
}) => {
    const {t} = useTranslation()
    // const [exportApprovalExecution] = useMutation<ExportApprovalExecutionMutation>(
    //     EXPORT_TASKS_EXECUTION,
    //     {
    //         context: {
    //             headers: {
    //                 "x-hasura-role": IPermissions.TASKS_READ,
    //             },
    //         },
    //     }
    // )

    const OMIT_FIELDS: string[] = []

    const filters: Array<ReactElement> = [
        <TextInput source="id" key="id_filter" label={t("tasksScreen.column.id")} />,
        <TextInput source="type" key="type_filter" label={t("tasksScreen.column.type")} />,
        <TextInput
            source="execution_status"
            key="status_filter"
            label={t("tasksScreen.column.execution_status")}
        />, 
    ]

    const actions: Action[] = [
        {
            icon: <Visibility />,
            action: onViewApproval,
        },
    ]


    return (
        <>
            <List
                actions={<ListActions withImport={false} withExport={false} />}
                resource="sequent_backend_tasks_execution"
                filters={filters}
                filter={{election_event_id: electionEventRecord?.id || undefined}}
                storeKey={false}
                sort={{field: "start_at", order: "DESC"}}
                perPage={10}
            >
                <ResetFilters />
                <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={<></>}>
                    <TextField source="id" />
                    <TextField source="name" />
                    <DateField
                        source="start_at"
                        showTime={true}
                        label={t("tasksScreen.column.start_at")}
                    />
                    <FunctionField
                        label={t("tasksScreen.column.execution_status")}
                        render={(record: any) => <StatusChip status={record.execution_status} />}
                    />
                    <ActionsColumn actions={actions} label={t("common.label.actions")} />
                </DatagridConfigurable>
            </List>
        </>
    )
}
