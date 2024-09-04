// SPDX-FileCopyrightText: 2024 Félix Robles <dev@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement} from "react"
import {
    List,
    TextInput,
    DateField,
    FunctionField,
    TextField,
    DatagridConfigurable,
    useNotify,
    Identifier,
} from "react-admin"
import {useTranslation} from "react-i18next"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {Visibility} from "@mui/icons-material"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {ListActions} from "@/components/ListActions"
import {StatusChip} from "@/components/StatusChip"

export interface ListTasksProps {
    onViewTask: (id: Identifier) => void
    electionEventRecord: Sequent_Backend_Election_Event
}
export const ListTasks: React.FC<ListTasksProps> = ({onViewTask, electionEventRecord}) => {
    const {t} = useTranslation()
    const [openExport, setOpenExport] = React.useState(false)
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
            action: onViewTask,
        },
    ]

    const handleExport = () => {
        console.log("EXPORT")
        setOpenExport(true)
    }

    return (
        <>
            <List
                actions={<ListActions withImport={false} doExport={handleExport} />}
                resource="sequent_backend_tasks_execution"
                filters={filters}
                filter={{election_event_id: electionEventRecord?.id || undefined}}
                sort={{field: "start_at", order: "DESC"}}
                perPage={10}
            >
                <DatagridConfigurable omit={OMIT_FIELDS} bulkActionButtons={<></>}>
                    <TextField source="id" />
                    <TextField source="type" />
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
